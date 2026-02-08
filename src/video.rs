use ffmpeg_the_third as ffmpeg;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

pub type SharedVideoFrame = Arc<Mutex<Option<VideoFrame>>>;

pub fn start_video_thread(
    device_path: &str,
) -> Result<SharedVideoFrame, Box<dyn std::error::Error>> {
    ffmpeg::init()?;

    let frame_buffer: SharedVideoFrame = Arc::new(Mutex::new(None));
    let frame_buffer_clone = Arc::clone(&frame_buffer);
    let device_path = device_path.to_string();

    thread::spawn(move || {
        if let Err(e) = video_capture_loop(&device_path, frame_buffer_clone) {
            eprintln!("Video capture error: {}", e);
        }
    });

    Ok(frame_buffer)
}

fn video_capture_loop(
    device_path: &str,
    frame_buffer: SharedVideoFrame,
) -> Result<(), Box<dyn std::error::Error>> {
    // Open the video device
    let mut ictx = ffmpeg::format::input(device_path).inspect_err(|e| {
        eprintln!("Failed to open device {}: {}", device_path, e);
    })?;

    println!("Device opened successfully: {}", device_path);

    // Find the video stream
    let input_stream = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or("Could not find video stream")?;
    let stream_index = input_stream.index();

    println!("Found video stream at index {}", stream_index);

    // Get decoder
    let mut context_decoder = ffmpeg::codec::context::Context::from_parameters(
        input_stream.parameters(),
    )
    .inspect_err(|e| {
        eprintln!("Failed to create codec context: {}", e);
    })?;

    // For rawvideo, we need to explicitly set parameters
    unsafe {
        let codec_params = context_decoder.as_mut_ptr();
        if !codec_params.is_null() {
            (*codec_params).width = 480;
            (*codec_params).height = 320;
            // YUYV422 pixel format
            (*codec_params).pix_fmt = ffmpeg::format::Pixel::YUYV422.into();
        }
    }

    let mut decoder = context_decoder.decoder().video().inspect_err(|e| {
        eprintln!("Failed to create decoder: {}", e);
    })?;

    println!(
        "Decoder format: {:?}, size: {}x{}",
        decoder.format(),
        decoder.width(),
        decoder.height()
    );

    // Setup scaler for RGB conversion
    let mut scaler = ffmpeg::software::scaling::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        ffmpeg::software::scaling::Flags::BILINEAR,
    )
    .inspect_err(|e| {
        eprintln!("Failed to create scaler: {}", e);
    })?;

    println!("Scaler created successfully, starting packet processing...");

    let width = decoder.width() as usize;
    let height = decoder.height() as usize;

    // Process packets
    let mut frame_count = 0;
    for result in ictx.packets() {
        if let Ok((stream, packet)) = result
            && stream.index() == stream_index {
                // Try to send packet, but skip if it fails (corrupted data)
                if let Err(e) = decoder.send_packet(&packet) {
                    eprintln!(
                        "Warning: Failed to send packet (frame {}): {}",
                        frame_count, e
                    );
                    continue;
                }

                let mut decoded = ffmpeg::util::frame::video::Video::empty();
                while decoder.receive_frame(&mut decoded).is_ok() {
                    let mut rgb_frame = ffmpeg::util::frame::video::Video::empty();
                    if let Err(e) = scaler.run(&decoded, &mut rgb_frame) {
                        eprintln!("Warning: Failed to scale frame {}: {}", frame_count, e);
                        continue;
                    }

                    // Copy frame data
                    let data = rgb_frame.data(0).to_vec();

                    // Update shared buffer
                    if let Ok(mut buffer) = frame_buffer.lock() {
                        *buffer = Some(VideoFrame {
                            data,
                            width,
                            height,
                        });
                    }

                    frame_count += 1;
                }
            }
    }

    Ok(())
}
