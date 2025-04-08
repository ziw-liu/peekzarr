use anyhow::{Ok, Result};
use clap::Parser;
use image::{DynamicImage, GrayImage};
use ndarray::prelude::*;
use ndarray_stats::interpolate::Nearest;
use ndarray_stats::QuantileExt;
use noisy_float::types::n64;
use std::path::PathBuf;
use std::sync::Arc;
use std::vec;
use zarrs::filesystem::FilesystemStore;

#[derive(Parser)]
#[command(version, about = "Peek into OME-Zarr images in the terminal.")]
struct Cli {
    /// Path to the OME-Zarr group containing arrays
    image_path: PathBuf,
    /// Name of the array (resolution level)
    #[arg(short, long, default_value = "/0")]
    array_name: String,
    /// Indices to slice non-XY dimensions
    #[arg(short, long, value_delimiter = ',', value_parser = clap::value_parser!(u64))]
    slice_indices: Option<Vec<u64>>,
    /// Maximum size to display in each dimension
    #[arg(short, long, default_value = "720")]
    crop_size: u64,
    /// Lower quantile for normalization
    #[arg(long, default_value = "0.001")]
    low: f64,
    /// Upper quantile for normalization
    #[arg(long, default_value = "0.999")]
    high: f64,
}

fn ensure_at_least_2d(array_shape: &[u64]) -> Result<()> {
    if array_shape.len() < 2 {
        anyhow::bail!("Array must have at least 2 dimensions");
    }
    Ok(())
}

fn push_index(start: &mut Vec<u64>, dimension: usize, value: u64) {
    println!("Slicing dimension {} at index {}", dimension, value);
    start.push(value);
}

fn start_and_shape(array_shape: &[u64], cli: &Cli) -> Result<(Vec<u64>, Vec<u64>)> {
    let ndims = array_shape.len();
    ensure_at_least_2d(array_shape)?;
    let ndims_to_be_sliced = ndims - 2;
    let mut start: Vec<u64> = vec![];
    if let Some(slice_indices) = &cli.slice_indices {
        if slice_indices.len() > ndims_to_be_sliced {
            anyhow::bail!(
                "Too many slice indices provided. Expected {} but got {}",
                ndims_to_be_sliced,
                slice_indices.len()
            );
        }
        for (i, slice_index) in slice_indices.iter().enumerate() {
            if *slice_index >= array_shape[i] {
                anyhow::bail!(
                    "Slice index {} is out of bounds for dimension {}",
                    slice_index,
                    i
                );
            }
            push_index(&mut start, i, *slice_index);
        }
    }
    for i in start.len()..(ndims_to_be_sliced) {
        push_index(&mut start, i, array_shape[i] / 2);
    }
    for _ in 0..2 {
        start.push(0);
    }
    let mut shape = vec![1; ndims];
    let axes = ["Y", "X"];
    for i in 0..2 {
        let full_size = array_shape[ndims_to_be_sliced + i];
        shape[ndims_to_be_sliced + i] = if cli.crop_size >= full_size {
            full_size
        } else {
            println!("Cropping dimension {:?} size {:?}", axes[i], cli.crop_size);
            cli.crop_size
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

fn read_image(cli: &Cli) -> Result<Array2<f32>> {
    let store = Arc::new(FilesystemStore::new(&cli.image_path)?);
    let array = zarrs::array::Array::open(store, &cli.array_name)?;
    let array_shape = array.shape();
    let (start, shape) = start_and_shape(&array_shape, cli)?;
    let subset = zarrs::array_subset::ArraySubset::new_with_start_shape(start, shape)?;
    let decoded = decode_subset(&array, &subset)?;
    Ok(decoded)
}

fn image_quantile(array: &Array2<f32>, q: f64) -> Result<f32> {
    let quantile = array
        .flatten()
        .quantile_axis_skipnan_mut(Axis(0), n64(q), &Nearest)?
        .into_scalar();
    Ok(quantile)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let decoded = read_image(&cli)?;
    let (rows, columns) = decoded.dim();
    let min = image_quantile(&decoded, cli.low)?;
    let max = image_quantile(&decoded, cli.high)?;
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
