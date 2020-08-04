use std::env;
use std::path::Path;

use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::{keyboard::Keycode, rect::Rect};
use sdl2::image::{LoadTexture, InitFlag};

use image::{ImageBuffer, Rgb};
use opensimplex_noise_rs::OpenSimplexNoise;

use rand::Rng;

const IMAGE_SIZE: [i32; 2] = [2048, 2048];
const WIN_SIZE: [i32; 2] = [512, 512];


fn sum_octaves(
    num_iterations: i32,
    point: (i32, i32),
    persistence: f64,
    scale: f64,
    low: f64,
    high: f64,
    noise_fn: impl Fn(f64, f64) -> f64,
) -> f64 {
    let mut max_amp = 0.0;
    let mut amp = 1.0;
    let mut freq = scale;
    let mut noise = 0.0;

    for _ in 0..num_iterations {
        noise += noise_fn(point.0 as f64 * freq, point.1 as f64 * freq) * amp;
        max_amp += amp;
        amp *= persistence;
        freq *= 2.0;
    }

    (noise / max_amp) * (high - low) / 2.0 + (high + low) / 2.0
}

fn color_to_array(color: Color) -> [u8; 3] {
    [color.r, color.g, color.b]
}

async fn generate_gradient() -> Vec<f32> {
    let mut gradient: Vec<f32> = vec![1.0; (IMAGE_SIZE[0] * IMAGE_SIZE[1]) as usize];

    for x in 0..IMAGE_SIZE[0] {
        for y in 0..IMAGE_SIZE[1] {
            let mut color_value: f32;

            let a = if x > (IMAGE_SIZE[0] / 2) {
                IMAGE_SIZE[0] - x
            } else {
                x
            };

            let b = if y > IMAGE_SIZE[1] / 2 {
                IMAGE_SIZE[1] - y
            } else {
                y
            };

            let smaller = std::cmp::min(a, b) as f32;
            color_value = smaller / (IMAGE_SIZE[0] as f32 / 2.0);

            color_value = 1.0 - color_value;
            color_value = color_value * color_value;

            gradient[get_id_from_pos(x, y)] = match color_value - 0.1 {
                x if x > 1.0 => 1.0,
                x if x < 0.0 => 0.0,
                x => x,
            };
        }
    }

    gradient
}

async fn generate_maps(gradient: &Vec<f32>) -> (Vec<f32>, Vec<f32>) {
    let mut rng = rand::thread_rng();

    let (mut height_map, mut biome_map) = futures::join!(
        generate_noise_map(rng.gen_range(0, std::i64::MAX), 0.004),
        generate_noise_map(rng.gen_range(0, std::i64::MAX), 0.007)
    );

    for x in 0..IMAGE_SIZE[0] {
        for y in 0..IMAGE_SIZE[1] {
            height_map[get_id_from_pos(x, y)] =
                height_map[get_id_from_pos(x, y)] * 1.1 - gradient[get_id_from_pos(x, y)] * 0.8;
            biome_map[get_id_from_pos(x, y)] =
                biome_map[get_id_from_pos(x, y)] - (0.1 - gradient[get_id_from_pos(x, y)]) * 0.4;
            if height_map[get_id_from_pos(x, y)] < 0.0 {
                height_map[get_id_from_pos(x, y)] = 0.0;
            }
            if biome_map[get_id_from_pos(x, y)] < 0.0 {
                biome_map[get_id_from_pos(x, y)] = 0.0;
            }
        }
    }

    (height_map, biome_map)
}

fn get_id_from_pos(x: i32, y: i32) -> usize {
    (x + IMAGE_SIZE[0] * y) as usize
}

async fn generate_noise_map(seed: i64, scale: f64) -> Vec<f32> {
    let noise_generator = OpenSimplexNoise::new(Some(seed));

    let mut map: Vec<f32> = vec![0.0; (IMAGE_SIZE[0] * IMAGE_SIZE[1]) as usize];
    for x in 0..IMAGE_SIZE[0] {
        for y in 0..IMAGE_SIZE[1] {
            let val = sum_octaves(16, (x, y), 0.5, scale, 0.0, 1.0, |x, y| {
                noise_generator.eval_2d(x, y)
            });

            map[get_id_from_pos(x, y)] = val as f32;
        }
    }
    map
}

enum Biomes {
    Grass,
    DeepWater,
    Water,
    Dirt,
    Sand,
    WetSand,
    DarkForest,
    HighDarkForest,
    LightForest,
    Mountain,
    HighMountain,
    Snow,
}

fn get_biome_color(biome: Biomes) -> Color {
    match biome {
        Biomes::Grass => Color::RGB(120, 157, 80),
        Biomes::Water => Color::RGB(9, 82, 198),
        Biomes::DeepWater => Color::RGB(0, 62, 178),
        Biomes::Dirt => Color::RGB(114, 98, 49),
        Biomes::Sand => Color::RGB(194, 178, 128),
        Biomes::WetSand => Color::RGB(164, 148, 99),
        Biomes::DarkForest => Color::RGB(60, 97, 20),
        Biomes::HighDarkForest => Color::RGB(40, 77, 0),
        Biomes::LightForest => Color::RGB(85, 122, 45),
        Biomes::Mountain => Color::RGB(140, 142, 123),
        Biomes::HighMountain => Color::RGB(160, 162, 143),
        Biomes::Snow => Color::RGB(235, 235, 235),
    }
}

enum AppState {
    Load,
    GenerateImage,
    View,
}

#[tokio::main]
async fn main() {
    let mut app_state = AppState::Load;

    let mut image = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(IMAGE_SIZE[0] as u32, IMAGE_SIZE[1] as u32);

    println!("Generating gradient...");
    let gradient = generate_gradient().await;
    println!("DONE");
    let mut height_map: Vec<f32> = Vec::new();
    let mut biome_map: Vec<f32> = Vec::new();


    'running: loop {
        match app_state {
            AppState::Load => {

                println!("Generating maps...");
                
                let (height, biome) = generate_maps(&gradient).await;
                height_map = height;
                biome_map = biome;
                
                println!("DONE");
                
                app_state = AppState::GenerateImage;
            }
            AppState::GenerateImage => {
                println!("Generating image...");
                for x in 0..IMAGE_SIZE[0] {
                    for y in 0..IMAGE_SIZE[1] {
                        let height = height_map[get_id_from_pos(x, y)];
                        let moisture = biome_map[get_id_from_pos(x, y)];

                        let biome = match (height, moisture) {
                            (a, _) if a < 0.39 => Biomes::DeepWater,
                            (a, _) if a < 0.42 => Biomes::Water,
                            (a, b) if a < 0.46 && b < 0.57 => Biomes::Sand,
                            (a, b) if a < 0.47 && b < 0.6 => Biomes::WetSand,
                            (a, b) if a < 0.47 && b >= 0.6 => Biomes::Dirt,
                            (a, b) if a > 0.54 && b < 0.43 && a < 0.62 => Biomes::Grass,
                            (a, b) if a < 0.62 && b >= 0.58 => Biomes::HighDarkForest,
                            (a, b) if a < 0.62 && b >= 0.49 => Biomes::DarkForest,
                            (a, _) if a >= 0.79 => Biomes::Snow,
                            (a, _) if a >= 0.74 => Biomes::HighMountain,
                            (a, b) if a >= 0.68 && b >= 0.10 => Biomes::Mountain,
                            _ => Biomes::LightForest,
                        };

                        let color = get_biome_color(biome);
                        let pixel = image.get_pixel_mut(x as u32, y as u32);
                        *pixel = image::Rgb(color_to_array(color));
                    }
                }
                image.save("output.png").unwrap();
                println!("DONE");
                app_state = AppState::View;
            },
            AppState::View => {
                let sdl_context = sdl2::init().unwrap();
                let video_subsystem = sdl_context.video().unwrap();

                let _image_context = sdl2::image::init(InitFlag::PNG | InitFlag::JPG)?;

                let window = video_subsystem
                    .window(
                        "noise visualization demo",
                        WIN_SIZE[0] as u32,
                        WIN_SIZE[1] as u32,
                    )
                    .position_centered()
                    .build()
                    .unwrap();
                let mut canvas = window.into_canvas().build().unwrap();

                let texture_creator = canvas.texture_creator();
                let texture = texture_creator.load_texture("output.png")?;

                canvas.copy(&texture, None, None)?;
                canvas.present();

                let mut event_pump = sdl_context.event_pump().unwrap();
                'view: loop {
                    canvas.clear();
                    for event in event_pump.poll_iter() {
                        match event {
                            Event::Quit { .. }
                            | Event::KeyDown {
                                keycode: Some(Keycode::Escape),
                                ..
                            } => {
                                break 'running;
                            },
                            Event::KeyDown {
                                keycode: Some(Keycode::Space),
                                ..
                            } => {
                                app_state = AppState::Load;
                                break 'view;
                            }
                            _ => {}
                        }
                    }
                    
                }
            }
        }
    }

    image.save("output.png").unwrap();
}
