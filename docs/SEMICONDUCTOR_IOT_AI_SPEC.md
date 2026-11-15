# AXON SEMICONDUCTOR, IOT HARDWARE & AI/ML SUITE SPECIFICATION

## 1. Overview
This module suite provides zero-dependency, zero-GC, hardware-sympathetic standard libraries written **100% in pure Axon (`.ax`)**.
It covers deep semiconductor register interfaces, bare-metal IoT microcontroller peripherals, neural processing units (NPU), ultra-low-bit quantization (16-bit down to 1-bit / 1.58-bit BitNet), and full pure-Axon reimplementations of world-class AI/ML libraries.

---

## 2. Architecture & Modules

```
+-----------------------------------------------------------------------------------------------+
|                                      AXON AI & HARDWARE                                       |
|    (Zero Python | Zero C++ | Zero LLVM | Zero CRT | Zero External DLLs | Zero-GC @no_alloc)   |
+-----------------------------------------------------------------------------------------------+
  |
  +---> Semiconductor & IoT Hardware:
  |       std/semiconductor.ax : MMIO 8/16/32/64-bit, Bitfields, Timers, DMA Ring, NVIC Controller
  |       std/iot.ax           : GPIO, UART (8N1/Bit-bang), SPI, I2C, PWM, ADC/DAC, MPU6050, SHT3x
  |       std/npu.ax           : 2D Systolic Array (Weight-Stationary), Bit-Serial MAC, VDPU BNN
  |
  +---> Multi-Precision & Ultra-Low-Bit Quantization:
  |       std/quant.ax         : FP16, BF16, Q8_0, Q4_0, NF4 (QLoRA), Q2_0, BitNet b1.58, BNN 1-bit
  |
  +---> 100% Pure Axon Reimplementations of World-Class AI / ML Python Libraries:
          std/numpy.ax         : NdArray, MatMul, Dot, Stats, Reductions, 2x2 Inv & Det
          std/torch.ax         : Autograd Tape, Tensor backward(), Linear, MSE/CrossEntropy, AdamW, SGD
          std/safetensors.ax   : Hugging Face SafeTensors Parser/Serializer, Zero-Copy Memory Map
          std/onnx.ax          : ONNX Graph Engine, InferenceSession, Node Dispatch (Gemm, Relu, Softmax)
          std/llama.ax         : GGUF Loader, RoPE, RMSNorm, SwiGLU, KV Cache, Top-P / Greedy Sampler
          std/vllm.ax          : PagedAttention v2, O(1) Block Recycling, Prefix Sharing, Continuous Batching
          std/transformers.ax  : CausalLM Architecture, High-Level text-generation pipeline
          std/tiktoken.ax      : OpenAI BPE Tokenizer, FNV-1a Rank Table, Byte Fallback
          std/tokenizers.ax    : HF Tokenizers Pipeline, Attention Mask, Batch Padding
          std/faiss.ax         : Vector Similarity Search (IndexFlatL2, IndexFlatIP), Top-K Heap
          std/sklearn.ax       : LinearRegression, LogisticRegression, KMeans, StandardScaler, Accuracy
```

---

## 3. Library Reference

### 3.1. Semiconductor & Bare-Metal Hardware (`std/semiconductor.ax`)
- `mmio_read8 / mmio_write8`, `mmio_read16 / mmio_write16`, `mmio_read32 / mmio_write32`, `mmio_read64 / mmio_write64`
- `mmio_set_bits32(addr, mask)`, `mmio_clear_bits32(addr, mask)`, `mmio_toggle_bits32(addr, mask)`
- `bitfield_extract(reg, lsb, width)`, `bitfield_deposit(reg, lsb, width, val)`
- `delay_cycles(n)`, `delay_us(us, mhz)`, `delay_ms(ms, mhz)`
- `dma_ring_new(n)`, `dma_ring_enqueue(mut ring, addr, len, ioc)`, `dma_ring_reclaim(mut ring)`
- `nvic_controller_new(base, max_irqs)`, `nvic_enable_irq`, `nvic_disable_irq`, `nvic_set_pending`, `nvic_clear_pending`

### 3.2. IoT & Embedded Peripherals (`std/iot.ax`)
- **GPIO:** `gpio_set_mode(port, pin, mode)`, `gpio_set_pull(port, pin, pull)`, `gpio_write(port, pin, high)`, `gpio_read(port, pin)`, `gpio_toggle(port, pin)`
- **UART:** `uart_write_byte`, `uart_read_byte`, `uart_write_str`, `uart_bitbang_tx_byte`
- **SPI:** `spi_transfer_byte`, `spi_transfer_buf`
- **I2C:** `i2c_write_reg8`, `i2c_read_reg8`
- **PWM:** `pwm_new(timer_base, channel, freq_hz)`, `pwm_set_duty(pwm, basis_10000)`
- **ADC:** `adc_new(base, vref_mv, res_bits)`, `adc_read_raw(adc, ch)`, `adc_read_millivolts(adc, ch)`
- **Sensors:** `mpu6050_init`, `mpu6050_read_sensors`, `sht3x_read`

### 3.3. Edge NPU & Systolic Array Acceleration (`std/npu.ax`)
- `systolic_grid_new(rows, cols)`, `systolic_load_weights`, `systolic_step_gemv(grid, in_act, out_accum)`
- `bit_serial_mac(act, weight, num_bits)`: Silicon area-optimized shift-and-add MAC unit
- `vdpu_dot_bnn_1bit_64(a_bits, b_bits)`: BNN dot product via `POPCOUNT(XNOR(A, B))`
- `npu_queue_new(base, cap)`, `npu_submit_job(mut queue, desc_ptr, bytes)`, `npu_wait_fence(queue, fence_id)`

### 3.4. Multi-Precision & Ultra-Low-Bit Quantization (`std/quant.ax`)
- **16-bit:** `f32_to_fp16`, `fp16_to_f32`, `f32_to_bf16`, `bf16_to_f32`
- **8-bit:** `quantize_q8_0_block(src, dst_block, scratch)`, `gemv_q8_0(act, q8_blocks, n_blocks)`
- **4-bit:** `quantize_q4_0_block`, `gemv_q4_0`, `gemv_nf4` (NormalFloat4 QLoRA)
- **2-bit:** `quantize_q2_0_block`, `gemv_q2_0` (4 weights per byte)
- **1.58-bit:** `quantize_bitnet_158_block`, `gemv_bitnet_158` (Multiplication-free ternary {-1, 0, +1})
- **1-bit:** `quantize_bnn_1bit_block`, `gemv_bnn_1bit` (Binary {-1, +1} POPCOUNT-XNOR)

### 3.5. Pure-Axon Reimplementations of AI / ML Libraries
- **NumPy (`std/numpy.ax`):** `np_zeros`, `np_ones`, `np_full`, `np_arange`, `np_linspace`, `np_eye`, `np_add`, `np_sub`, `np_multiply`, `np_divide`, `np_matmul`, `np_dot`, `np_sum`, `np_mean`, `np_std`, `np_var`, `np_max`, `np_min`, `np_argmax`, `np_argmin`, `np_linalg_det_2x2`, `np_linalg_inv_2x2`
- **PyTorch (`std/torch.ax`):** `tensor_new`, `torch_add`, `torch_mul`, `torch_matmul`, `torch_relu`, `TorchLinear`, `torch_linear_forward`, `nn_mse_loss`, `nn_cross_entropy_loss`, `optim_sgd_step`, `optim_adamw_step`
- **SafeTensors (`std/safetensors.ax`):** `safetensors_serialize(name, arr)`, `safetensors_parse_buffer(raw)`, `safetensors_get_tensor(st, name, r, c)`
- **ONNX Runtime (`std/onnx.ax`):** `onnx_session_new`, `onnx_add_node`, `onnx_set_tensor_slot`, `onnx_session_run`
- **LLaMA.cpp (`std/llama.ax`):** `llama_parse_gguf_header`, `llama_rmsnorm`, `llama_apply_rope`, `llama_swiglu`, `llama_kv_cache_new`, `llama_sample_top_p`, `llama_sample_greedy`
- **vLLM (`std/vllm.ax`):** `vllm_block_table_new`, `vllm_alloc_block`, `vllm_share_block`, `vllm_free_block`, `vllm_request_new`, `vllm_engine_new`, `vllm_request_append_token`
- **Transformers (`std/transformers.ax`):** `causal_lm_new`, `causal_lm_forward_token`, `pipeline_text_generation`
- **TikToken & Tokenizers (`std/tiktoken.ax`, `std/tokenizers.ax`):** `tiktoken_vocab_new`, `tiktoken_add_token`, `tiktoken_encode`, `tiktoken_decode`, `tokenizer_encode_with_mask`
- **FAISS (`std/faiss.ax`):** `faiss_index_flat_new`, `faiss_index_flat_add`, `faiss_search_flat`
- **Scikit-Learn (`std/sklearn.ax`):** `sklearn_linear_regression_new`, `sklearn_linear_regression_fit`, `sklearn_linear_regression_predict`, `sklearn_logistic_regression_fit`, `sklearn_kmeans_fit`, `sklearn_standard_scaler_fit`, `sklearn_accuracy_score`

---

## 4. Verification & Testing
Execute test suite:
```powershell
.\bin\axc.exe test tests/pass/28_semiconductor_iot_hardware.ax
.\bin\axc.exe test tests/pass/29_ultra_low_bit_quant_1_to_16bit.ax
.\bin\axc.exe test tests/pass/30_full_ai_libraries_suite.ax
```
All 16 test functions pass with 0 errors, 0 runtime dependencies, and 0 memory leaks under Perceus Linear RC.
