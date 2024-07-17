use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::f64::consts::PI;

use anyhow::Result;
use candle_core::{op, scalar::TensorScalar, DType, Device, Shape, Tensor, Var};
use candle_nn::{Optimizer, VarMap};
use image::{DynamicImage, GenericImageView, ImageEncoder, ImageFormat};
use rayon::iter::IntoParallelIterator;
use std::io::Write;
use tokio::sync::mpsc::Receiver;
use tron_app::tron_components::div::update_and_send_div_with_context;
use tron_app::tron_components::{TnAsset, TnContext, TnServiceRequestMsg};

use candle_nn::init::Init::{Const, Uniform};

use crate::{IMAGE_OUTPUT_AREA, INPUT_IMAGE_AREA};
use data_encoding::BASE64;
use rayon::prelude::*;

fn render(gs: &Var, grids_xy: &[Tensor], device: &Device) -> Result<Vec<Tensor>> {
    let x = &grids_xy[0];
    let y = &grids_xy[1];

    let size = gs.shape().dims()[0];
    let gs_vec = (0..size)
        .flat_map(|idx| -> Result<_> {
            let gs0 = &gs.get(idx)?;
            let ux0 = gs0.get(0)?;
            let uy0 = gs0.get(1)?;
            let a0 = gs0.get(2)?;
            let b0 = gs0.get(3)?;
            let c0 = gs0.get(4)?;
            let red0 = gs0.get(5)?;
            let green0 = gs0.get(6)?;
            let blue0 = gs0.get(7)?;
            let alpha0 = gs0.get(8)?;
            Ok((ux0, uy0, a0, b0, c0, red0, green0, blue0, alpha0))
        })
        .collect::<Vec<_>>();
    
    // the par_iter does not help much. some lock issue?
    let res = gs_vec
        .into_par_iter()
        .flat_map(
            |(ux0, uy0, a0, b0, c0, red0, green0, blue0, alpha0)| -> Result<_> {
                let a0 = &a0
                    .erf()?
                    .add(&Tensor::from_slice(&[1.0_f32], (), device)?)?
                    .mul(&Tensor::from_slice(&[2.0_f32], (), device)?)?;

                let b0 = &b0
                    .erf()?
                    .add(&Tensor::from_slice(&[1.0_f32], (), device)?)?
                    .mul(&Tensor::from_slice(&[2.0_f32], (), device)?)?;

                let dx = x.broadcast_sub(&ux0)?;
                let dy = y.broadcast_sub(&uy0)?;

                let sin = c0.sin()?;
                let cos = c0.cos()?;
                let rdx = dx.broadcast_mul(&cos)?.add(&dy.broadcast_mul(&sin)?)?;
                let rdy = dx
                    .broadcast_mul(&sin.neg()?)?
                    .add(&dy.broadcast_mul(&cos)?)?;

                let rdxsq = &rdx.sqr()?.broadcast_mul(a0)?;
                let rdysq = &rdy.sqr()?.broadcast_mul(b0)?;

                let v = &rdxsq.add(rdysq)?.neg()?.exp()?;
                let r0 = v.broadcast_mul(&red0)?.broadcast_mul(&alpha0)?;
                let g0 = v.broadcast_mul(&green0)?.broadcast_mul(&alpha0)?;
                let b0 = v.broadcast_mul(&blue0)?.broadcast_mul(&alpha0)?;

                Ok([r0, g0, b0])
            },
        )
        .collect::<Vec<_>>();

    let r_s = res.iter().map(|v| v[0].clone()).collect::<Vec<_>>();
    let g_s = res.iter().map(|v| v[1].clone()).collect::<Vec<_>>();
    let b_s = res.iter().map(|v| v[2].clone()).collect::<Vec<_>>();

    let r_s = Tensor::stack(&r_s[..], 0)?.sum(0)?;
    let g_s = Tensor::stack(&g_s[..], 0)?.sum(0)?;
    let b_s = Tensor::stack(&b_s[..], 0)?.sum(0)?;

    Ok(vec![r_s.clone(), g_s.clone(), b_s.clone()])
}

use std::io::Cursor;
pub async fn gs_service(context: TnContext, mut rx: Receiver<TnServiceRequestMsg>) {
    while let Some(r) = rx.recv().await {
        let context = context.clone();
        if r.request == "process_image" {
            let _ = r.response.send("OK".to_string());

            if let TnAsset::String(filename) = r.payload {
                let asset_ref = context.get_asset_ref().await;
                let guard = asset_ref.read().await;
                let asset = guard.get("upload").unwrap();
                let image_data = if let TnAsset::HashMapVecU8(h) = asset {
                    h.get(&filename)
                } else {
                    None
                };
                if let Some(image_data) = image_data {
                    let image = image::load_from_memory(image_data).expect("can't load image");
                    let image = image.resize(128, 128, image::imageops::FilterType::Gaussian);

                    let mut buf = Cursor::new(vec![]);
                    image.write_to(&mut buf, ImageFormat::Png).unwrap();
                    let image_b64 = BASE64.encode(&buf.into_inner());
                    update_and_send_div_with_context(
                        &context,
                        INPUT_IMAGE_AREA,
                        &format!(r##"<img src="data:image/png;base64,{}" style="width: 55vw; min-width: 240px;"/>"##, image_b64),
                    )
                    .await;
                    let _ = gs_fit(&context.clone(), &image).await;
                };
            }
        }
    }
}

pub async fn gs_fit(context: &TnContext, ref_img: &DynamicImage) -> Result<()> {
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let dims = ref_img.dimensions();
    //println!("dimensions {:?}", ref_img.dimensions());
    //println!("{:?}", ref_img.color());

    //let device = Device::new_cuda(0).unwrap();
    //let device = Device::new_metal(0).unwrap();
    let device = Device::Cpu;

    let mut ref_r = Vec::new();
    let mut ref_g = Vec::new();
    let mut ref_b = Vec::new();

    for y0 in 0..dims.1 {
        let mut r = Vec::new();
        let mut g = Vec::new();
        let mut b = Vec::new();
        for x0 in 0..dims.0 {
            let p = ref_img.get_pixel(x0, y0).0;
            r.push(p[0] as f32 / 256.0);
            g.push(p[1] as f32 / 256.0);
            b.push(p[2] as f32 / 256.0);
        }
        ref_r.extend(r.into_iter());
        ref_g.extend(g.into_iter());
        ref_b.extend(b.into_iter());
    }

    let ref_r = Tensor::from_vec(ref_r, (dims.1 as usize, dims.0 as usize), &device).unwrap();
    let ref_g = Tensor::from_vec(ref_g, (dims.1 as usize, dims.0 as usize), &device).unwrap();
    let ref_b = Tensor::from_vec(ref_b, (dims.1 as usize, dims.0 as usize), &device).unwrap();
    //println!("REF_SHAPE {:?}", ref_r.shape());
    //let size = 2500;
    let size = 1024;

    let var_init = [
        ("ux", 0.0_f32, dims.0 as f32),
        ("uy", 0.0_f32, dims.1 as f32),
        ("a", -1.5_f32, -1.0_f32),
        ("b", -1.5_f32, -1.0_f32),
        ("c", -PI as f32, PI as f32),
        ("red", 0.00_f32, 0.05_f32),
        ("green", 0.00_f32, 0.05_f32),
        ("blue", 0.00_f32, 0.05_f32),
        ("alpha", 0.00_f32, 1.0_f32),
    ];
    let r = var_init
        .into_iter()
        .map(|(_name, lo, up)| Tensor::rand(lo, up, (size,), &device).unwrap())
        .collect::<Vec<_>>();
    let r = Tensor::stack(&r, 0).unwrap().t().unwrap();

    let var = Var::from_tensor(&r).unwrap();
    let mut opt = candle_nn::AdamW::new_lr(vec![var.clone()], 0.1)?;

    let x = Tensor::arange(0.0_f32, dims.0 as f32, &device)?;
    let y = Tensor::arange(0.0_f32, dims.1 as f32, &device)?;

    let grids_xy = Tensor::meshgrid(&[&x, &y], true)?;
    for i in 0..51 {
        let e = render(&var, &grids_xy, &device)?;

        let loss_r = &e[0].sub(&ref_r)?.abs()?.sum_all()?;
        let loss_g = &e[1].sub(&ref_g)?.abs()?.sum_all()?;
        let loss_b = &e[2].sub(&ref_b)?.abs()?.sum_all()?;

        let loss = &loss_r.add(loss_g)?.add(loss_b)?;
        let gradient_store = loss.backward()?;
        println!("{} loss: {}", i, loss.to_vec0::<f32>().unwrap());
        opt.step(&gradient_store)?;

        if i % 2 == 0 {
            let r = e[0].to_vec2::<f32>()?;
            let g = e[1].to_vec2::<f32>()?;
            let b = e[2].to_vec2::<f32>()?;

            let mut imgbuf = image::ImageBuffer::new(dims.0, dims.1);

            for (x0, y0, pixel) in imgbuf.enumerate_pixels_mut() {
                let rr = r[y0 as usize][x0 as usize];
                let gg = g[y0 as usize][x0 as usize];
                let bb = b[y0 as usize][x0 as usize];
                let r = (256.0 * rr) as u8;
                let g = (256.0 * gg) as u8;
                let b = (256.0 * bb) as u8;
                *pixel = image::Rgb([r, g, b]);
            }
            {
                let mut buf = Cursor::new(vec![]);
                imgbuf.write_to(&mut buf, ImageFormat::Png).unwrap();
                let image_b64 = BASE64.encode(&buf.into_inner());
                update_and_send_div_with_context(
                    context,
                    IMAGE_OUTPUT_AREA,
                    &format!(r##"<img src="data:image/png;base64,{}" style="width: 55vw; min-width: 240px;"/>"##, image_b64),
                )
                .await;
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        };
    }
    Ok(())
}
