//! The ``audio`` module of ``ogc-rs``.
//!
//! This module implements a safe wrapper around the audio functions found in ``audio.h``.

use crate::ffi;
use alloc::boxed::Box;
use core::{convert::TryFrom, mem, ptr};
use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Represents the audio service.
/// No audio control can be done until an instance of this struct is created.
/// This service can only be created once!
pub struct Audio;

/// The play state of the ``audio`` service.
#[derive(IntoPrimitive, TryFromPrimitive, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PlayState {
    Started = 1,
    Stopped = 0,
}

/// The sample rate of the ``audio`` service.
#[derive(IntoPrimitive, TryFromPrimitive, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SampleRate {
    FortyEightKhz = 1,
    ThirtySixKhz = 0,
}

/// Implementation of the audio service.
impl Audio {
    /// Initialization of the audio service.
    pub fn init() -> Self {
        unsafe {
            // For now this is a mutable null pointer.
            // libogc is fine with this, but this should be changed in the future.
            ffi::AUDIO_Init(ptr::null_mut());

            Self
        }
    }

    /// Initialize an audio DMA transfer.
    fn init_dma(data: &[u8]) {
        unsafe {
            // libogc has strict restrictions on data alignment and length.
            assert_eq!(
                32,
                mem::align_of_val(data),
                "Data is not aligned correctly."
            );
            assert_eq!(0, data.len() % 32, "Data length is not a multiple of 32.");
            // TODO: Can we somehow comply with strict provenance here?
            ffi::AUDIO_InitDMA(data.as_ptr() as u32, data.len() as u32);
        }
    }

    /// Start the audio DMA operation.
    ///
    /// Starts to transfer the data from main memory to the audio interface through DMA.
    /// This call should follow the call to ``init_dma`` which is used to initialize DMA transfers.
    fn start_dma() {
        unsafe {
            ffi::AUDIO_StartDMA();
        }
    }

    /// Stop the previously started audio DMA operation.
    fn stop_dma() {
        unsafe {
            ffi::AUDIO_StopDMA();
        }
    }

    /// Register a user callback function for the ``audio`` streaming interface.
    fn register_stream_callback<F>(callback: Box<F>)
    where
        F: Fn(u32),
    {
        // TODO: Check if this implementation can be changed.
        let ptr = Box::into_raw(callback);

        unsafe {
            let code: extern "C" fn(smp_cnt: u32) = mem::transmute(ptr);
            // TODO: Do something with the returned callback.
            let _ = ffi::AUDIO_RegisterStreamCallback(Some(code));
        }
    }

    /// Register a user callback function for the audio DMA interface.
    ///
    /// This callback will be called whenever the audio DMA requests new data.
    /// Internally the DMA buffers are double buffered.
    fn register_dma_callback<F>(callback: Box<F>)
    where
        F: Fn(),
    {
        // TODO: Check if this implementation can be changed.
        let ptr = Box::into_raw(callback);

        unsafe {
            let code: extern "C" fn() = mem::transmute(ptr);
            // TODO: Do something with the returned callback.
            let _ = ffi::AUDIO_RegisterDMACallback(Some(code));
        }
    }

    /// Get the count of bytes, left to play, from the audio DMA interface.
    fn get_dma_bytes_left() -> u32 {
        unsafe { ffi::AUDIO_GetDMABytesLeft() }
    }

    /// Get the audio DMA flag.
    fn get_dma_enable_flag() -> u16 {
        unsafe { ffi::AUDIO_GetDMAEnableFlag() }
    }

    /// Get the DMA transfer length configured in the audio DMA interface.
    fn get_dma_length() -> u32 {
        unsafe { ffi::AUDIO_GetDMALength() }
    }

    /// Get the main memory address for the DMA operation.
    fn get_dma_address() -> u32 {
        unsafe { ffi::AUDIO_GetDMAStartAddr() }
    }

    /// Reset the stream sample count register.
    fn reset_sample_count() {
        unsafe {
            ffi::AUDIO_ResetStreamSampleCnt();
        }
    }

    /// Set the sample count for the stream trigger.
    fn set_trigger_count(count: u32) {
        unsafe {
            ffi::AUDIO_SetStreamTrigger(count);
        }
    }

    /// Get streaming sample rate.
    fn get_samplerate() -> SampleRate {
        let r = unsafe { ffi::AUDIO_GetStreamSampleRate() };
        SampleRate::try_from(r).unwrap()
    }

    /// Get the sampling rate for the DSP interface.
    fn get_dsp_samplerate() -> SampleRate {
        let r = unsafe { ffi::AUDIO_GetDSPSampleRate() };
        SampleRate::try_from(r).unwrap()
    }

    /// Set the sample rate for the streaming audio interface.
    fn set_samplerate(samplerate: SampleRate) {
        unsafe {
            ffi::AUDIO_SetStreamSampleRate(samplerate.into());
        }
    }

    /// Set the sampling rate for the DSP interface.
    fn set_dsp_samplerate(samplerate: SampleRate) {
        // TODO: Check implementation.
        let sample_rate: u32 = samplerate.into();

        unsafe {
            ffi::AUDIO_SetDSPSampleRate(sample_rate as u8);
        }
    }

    /// Get the play state from the streaming audio interface.
    fn get_playstate() -> PlayState {
        let r = unsafe { ffi::AUDIO_GetStreamPlayState() };
        PlayState::try_from(r).unwrap()
    }

    /// Set the play state for the streaming audio interface.
    fn set_playstate(playstate: PlayState) {
        unsafe {
            ffi::AUDIO_SetStreamPlayState(playstate.into());
        }
    }

    /// Get streaming volume on the left channel.
    fn get_volume_left() -> u8 {
        unsafe { ffi::AUDIO_GetStreamVolLeft() }
    }

    /// Set streaming volume on the left channel.
    fn set_volume_left(volume: u8) {
        unsafe { ffi::AUDIO_SetStreamVolLeft(volume) }
    }

    /// Get streaming volume on the right channel.
    fn get_volume_right() -> u8 {
        unsafe { ffi::AUDIO_GetStreamVolRight() }
    }

    /// Set streaming volume on the right channel.
    fn set_volume_right(volume: u8) {
        unsafe { ffi::AUDIO_SetStreamVolRight(volume) }
    }
}
