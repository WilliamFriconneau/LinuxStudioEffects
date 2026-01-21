use anyhow::{Result, Context};
use ort::{GraphOptimizationLevel, Session, SessionBuilder, execution_providers::*};
use ndarray::{Array4, ArrayView3};
use std::sync::Arc;
use log::{info, warn, error};

pub struct SegmentationEngine {
    session: Session,
}

impl SegmentationEngine {
    pub fn new(model_path: &str, backend: &str) -> Result<Self> {
        info!("Initializing SegmentationEngine with backend: {}", backend);

        // Configure Execution Providers based on backend
        let mut builder = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?;

        // Note: For VitisAI (AMD NPU) and TensorRT (Nvidia), the libraries must be present in the system/path.
        // The 'ort' crate with 'load-dynamic' feature will try to load them if requested.
        
        match backend {
            "nvidia" => {
                info!("Attempting to register CUDA/TensorRT Execution Providers...");
                // TensorRT first, then CUDA fallback
                if let Err(e) = builder.set_execution_providers([
                    TensorRTExecutionProvider::default().build(),
                    CUDAExecutionProvider::default().build(),
                ]) {
                   warn!("Failed to register Nvidia providers: {}. Falling back to CPU.", e);
                }
            }
            "intel" => {
                info!("Attempting to register OpenVINO Execution Provider...");
                 if let Err(e) = builder.set_execution_providers([
                    OpenVINOExecutionProvider::default().build(),
                ]) {
                   warn!("Failed to register OpenVINO: {}. Falling back to CPU.", e);
                }
            }
            "amd" | "npu" => {
                info!("Attempting to register AMD/NPU providers (VitisAI / ROCm)...");
                // VitisAI is often "VitisAIExecutionProvider" or configured via config options.
                // Since explicit VitisAI struct might be missing in some ort versions or requires features,
                // we try generic registration or fallback to OpenVINO (which supports some AMD hardware) or CPU.
                // For "npu", CoreML is good for Mac (testing), VitisAI for Linux/AMD.
                
                #[cfg(target_os = "macos")]
                {
                     let _ = builder.set_execution_providers([
                        CoreMLExecutionProvider::default().build(),
                    ]);
                }
                #[cfg(target_os = "linux")]
                {
                    // Generic attempt for VitisAI or ROCm if available available via shared libs
                    // If ORT dynamic lib has them compiled in, they are available.
                    // We can also try OpenVINO which works on some AMD CPUs/iGPUs.
                    // For now, we rely on the dynamic loader picking up what's available or falling back to CPU.
                    // Ideally:
                    // builder.with_execution_providers([VitisAIExecutionProvider::default().build()])
                }
            }
            _ => { // cpu or auto
                 info!("Using default CPU execution provider.");
            }
        }

        let session = builder.with_model_from_file(model_path)
            .context("Failed to load ONNX model. Check if 'segmentation.onnx' exists in ~/.config/linux-studio-effects/models/")?;

        Ok(Self { session })
    }

    // Input: 256x256 RGB image (HWC or CHW? usually ONNX expects CHW 1x3x256x256)
    // Output: 256x256 Mask (1x1x256x256)
    pub fn infer(&self, input_tensor: ArrayView3<u8>) -> Result<Vec<u8>> {
        // Preprocessing: u8 [0-255] -> f32 [0.0-1.0] and Normalize? 
        // Depends on model. Let's assume standard: (Values - Mean) / Std or just / 255.0.
        // Input: 256x256x3 (HWC) from GStreamer Buffer?
        // Convert to 1x3x256x256 (NCHW)
        
        // This is the expensive part (Memory Copy/Transform). 
        // Zero-copy is hard between GStreamer and Ndarray without unsafe.
        
        let (h, w, c) = input_tensor.dim();
        // Assume 256x256x3
        
        let mut tensor_data = Array4::<f32>::zeros((1, 3, h, w));
        
        // Simple manual transpose & normalization loop
        // Optimize later with SIMD or GStreamer 'videoconvert' to planar format
        for y in 0..h {
            for x in 0..w {
                let pixel = input_tensor.slice(ndarray::s![y, x, ..]);
                tensor_data[[0, 0, y, x]] = pixel[0] as f32 / 255.0; // R
                tensor_data[[0, 1, y, x]] = pixel[1] as f32 / 255.0; // G
                tensor_data[[0, 2, y, x]] = pixel[2] as f32 / 255.0; // B
            }
        }

        let inputs = ort::inputs!["input" => tensor_data.view()]?;
        
        let outputs = self.session.run(inputs)?;
        
        // Get output "output" or index 0
        let output_tensor = outputs["output"].extract_tensor::<f32>()?;
        let output_view = output_tensor.view(); // 1x1x256x256 or 1x2x256x256 (classes)
        
        // Post-process: Extract mask (class 1 or prob > 0.5)
        // Map back to u8 256x256 (Hardware scale up will handle the rest)
        
        let mut mask_data = Vec::with_capacity(h * w);
        
        for y in 0..h {
            for x in 0..w {
                // Assuming Output is 1x1xHxW (Probability of FG) or 1xCxHxW (argmax needed)
                // Let's assume binary segmentation 1x1xHxW output with logit or prob.
                
                // Safety check dims
                // let val = output_view[[0, 0, y, x]]; 
                // Using get to be safe or assuming fixed shape
                // For MVP, just fill with 255 if > 0.5, else 0
                
                // Dummy pass-through if model logic is complex, but let's try reading:
                if let Some(val) = output_view.get([0, 0, y, x]) {
                    mask_data.push(if *val > 0.5 { 255 } else { 0 });
                } else {
                    mask_data.push(0);
                }
            }
        }
        
        Ok(mask_data)
    }
}
