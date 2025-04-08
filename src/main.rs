use anyhow::{Ok, Result};
use clap::Parser;
use image::{DynamicImage, GrayImage};
use ndarray::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use zarrs::filesystem::FilesystemStore;

#[derive(Parser)]
#[command(version, about = "Peek into OME-Zarr images in the terminal")]
struct Cli {
    image_path: PathBuf,
    #[arg(short, long, default_value = "/0")]
    array_name: String,
    #[arg(short, long, default_value = "720")]
    crop_size: u64,
}

fn ensure_at_least_2d(array_shape: &[u64]) -> Result<()> {
    if array_shape.len() < 2 {
        anyhow::bail!("Array must have at least 2 dimensions");
    }
    Ok(())
}

fn start_and_shape(array_shape: &[u64], crop_size: u64) -> Result<(Vec<u64>, Vec<u64>)> {
    let ndims = array_shape.len();
    ensure_at_least_2d(array_shape)?;
    let start: Vec<u64> = vec![0; ndims];
    let mut shape = vec![1; ndims];
    let axes = ["Y", "X"];
    for i in 0..2 {
        let full_size = array_shape[ndims - 2 + i];
        shape[ndims - 2 + i] = if crop_size >= full_size {
            full_size
        } else {
            println!("Cropping dimension {:?} size {:?}", axes[i], crop_size);
            crop_size
        };
    }
    Ok((start, shape))
}

fn decode_subset(
    array: &zarrs::array::Array<FilesystemStore>,
    subset: &zarrs::array_subset::ArraySubset,
) -> Result<Array2<f32>> {
    use zarrs::array::DataType;
    let dtype = array.data_type();
    let decoded = match dtype {
        DataType::Int8 => array
            .retrieve_array_subset_ndarray::<i8>(&subset)?
            .mapv(|x| x as f32),
        DataType::Int16 => array
            .retrieve_array_subset_ndarray::<i16>(&subset)?
            .mapv(|x| x as f32),
        DataType::Int32 => array
            .retrieve_array_subset_ndarray::<i32>(&subset)?
            .mapv(|x| x as f32),
        DataType::Int64 => array
            .retrieve_array_subset_ndarray::<i64>(&subset)?
            .mapv(|x| x as f32),
        DataType::UInt8 => array
            .retrieve_array_subset_ndarray::<u8>(&subset)?
            .mapv(|x| x as f32),
        DataType::UInt16 => array
            .retrieve_array_subset_ndarray::<u16>(&subset)?
            .mapv(|x| x as f32),
        DataType::UInt32 => array
            .retrieve_array_subset_ndarray::<u32>(&subset)?
            .mapv(|x| x as f32),
        DataType::Float32 => array.retrieve_array_subset_ndarray::<f32>(&subset)?,
        DataType::Float64 => array
            .retrieve_array_subset_ndarray::<f64>(&subset)?
            .mapv(|x| x as f32),
        _ => anyhow::bail!("Unsupported data type: {:?}", dtype),
    };
    let shape = decoded.shape();
    let y = shape[shape.len() - 2];
    let x = shape[shape.len() - 1];
    let reshaped = decoded.to_shape((y, x))?.to_owned();
    Ok(reshaped)
}

fn read_image(cli: Cli) -> Result<Array2<f32>> {
    let store = Arc::new(FilesystemStore::new(cli.image_path)?);
    let array = zarrs::array::Array::open(store, &cli.array_name)?;
    let array_shape = array.shape();
    let (start, shape) = start_and_shape(&array_shape, cli.crop_size)?;
    let subset = zarrs::array_subset::ArraySubset::new_with_start_shape(start, shape)?;
    let decoded = decode_subset(&array, &subset)?;
    Ok(decoded)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let min = 100.0;
    let max = 800.0;
    let decoded = read_image(cli)?;
    let (rows, columns) = decoded.dim();
    let normalized = decoded.mapv(|x| ((x.clamp(min, max) - min) / (max - min) * 255.0) as u8);
    let data = normalized
        .as_standard_layout()
        .to_owned()
        .into_raw_vec_and_offset()
        .0;
    let image = GrayImage::from_raw(columns as u32, rows as u32, data)
        .ok_or_else(|| anyhow::anyhow!("Failed to create image from raw data"))?;
    let image = DynamicImage::ImageLuma8(image);
    let conf = viuer::Config {
        absolute_offset: false,
        ..Default::default()
    };
    viuer::print(&image, &conf)?;
    Ok(())
}
