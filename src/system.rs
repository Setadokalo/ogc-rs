//! The ``system`` module of ``ogc-rs``.
//!
//! This module implements a safe wrapper around the OS functions found in ``system.h``.

use crate::{ffi, video::RenderConfig, OgcError, Result};
use alloc::boxed::Box;
use ogc_sys::TB_TIMER_CLOCK;
use core::{ffi::c_void, marker::PhantomData, mem, ptr, time::Duration};
use num_enum::{TryFromPrimitive, IntoPrimitive};

/// Represents the system service.
/// The initialization of this service is done in the crt0 startup code.
pub struct System;

#[derive(Debug)]
// T must be Send since we send it across threads, but it only needs to be Sync
// if we don't take ownership (since we're then sharing references across threads)
pub struct Alarm<'a, T: Send>(u32, Option<Box<T>>, PhantomData<&'a u8>);

// Generic impl for all T, Sync or not
impl<'a, T: Send> Alarm<'a, T> {

    /// Cancel the alarm, but do not remove from the list of contexts.
    pub fn cancel_alarm(&mut self) {
        let r = unsafe { ffi::SYS_CancelAlarm(self.0) };

        assert!(r == 0, "Failed to set periodic alarm - shouldn't be possible without ogc_sys raw usage");
    }

    /// Set the alarm parameters for a one-shot alarm, add to the list of alarms and start.
    /// 
    /// This function can softlock the console if called with a `delay` of 0 seconds
    /// on an alarm that's already set.
    pub fn set_owning(
        &mut self,
        delay: Duration, 
        callback: extern "C" fn(alarm: u32, cb_arg: &'a T), 
        cb_arg: T
    ) {
        // Convert Duration to timespec
        let timespec: *const ffi::timespec = &ffi::timespec {
            tv_sec: delay.as_secs() as i64,
            tv_nsec: delay.as_nanos() as i32,
        };
        self.1 = Some(Box::new(cb_arg));
        // Option<&T> is ABI compatible with *mut T.
        // *mut T is ABI compatible with *mut U
        // if <T as Pointee>::Metadata == <U as Pointee>::Metadata.
        // For all Sized types, <T as Pointee>::Metadata = ().
        let callback = unsafe { 
            mem::transmute::<
                extern "C" fn(alarm: u32, cb_arg: &'a T),
                extern "C" fn(alarm: u32, cb_arg: *mut c_void)
            >(callback)
        };
        let r = unsafe { ffi::SYS_SetAlarm(
            self.0, 
            timespec, 
            Some(callback), 
            ((&**self.1.as_ref().unwrap()) as *const T)
                .cast_mut()
                .cast::<c_void>(),
        ) };

        assert!(r == 0, "Failed to set alarm - shouldn't be possible without ogc_sys raw usage");
    }

    /// Set the alarm parameters for a periodic alarm, add to the list of alarms and start.
    /// The alarm and interval persists as long as SYS_CancelAlarm() isn't called.
    /// 
    /// This function can softlock the console if called with a `delay` of 0 seconds
    /// on an alarm that's already set.
    pub fn set_periodic_owning(
        &mut self,
        start_delay: Duration,
        period: Duration,
        callback: extern "C" fn(alarm: u32, cb_arg: &'a T),
        callback_data: T,
    ) {
        // Convert Duration to timespec
        let timespec_start: *const ffi::timespec = &ffi::timespec {
            tv_sec: start_delay.as_secs() as i64,
            tv_nsec: start_delay.as_nanos() as i32,
        };

        let timespec_period: *const ffi::timespec = &ffi::timespec {
            tv_sec: period.as_secs() as i64,
            tv_nsec: period.as_nanos() as i32,
        };
        self.1 = Some(Box::new(callback_data));
        // See set_alarm for safety explanation.
        let callback = unsafe { 
            mem::transmute::<
                extern "C" fn(alarm: u32, cb_arg: &'a T),
                extern "C" fn(alarm: u32, cb_arg: *mut c_void)
            >(callback)
        };
        let r = unsafe {
            ffi::SYS_SetPeriodicAlarm(
                self.0,
                timespec_start,
                timespec_period,
                Some(callback),
                ((&**self.1.as_ref().unwrap()) as *const T)
                    .cast_mut()
                    .cast::<c_void>(),
            )
        };

        assert!(r == 0, "Failed to set periodic alarm - shouldn't be possible without ogc_sys raw usage");
    }
}
// T that are Sync don't need us to take ownership to share
impl<'a, T: Send + Sync> Alarm<'a, T> {
    /// Set the alarm parameters for a one-shot alarm, add to the list of alarms and start.
    /// 
    /// This function can softlock the console if called with a `delay` of 0 seconds
    /// on an alarm that's already set.
    pub fn set(
        &mut self,
        delay: Duration, 
        callback: extern "C" fn(alarm: u32, cb_arg: Option<&'a T>), 
        cb_arg: impl Into<Option<&'a T>>
    ) {
        let cb_arg = cb_arg.into();
        // Convert Duration to timespec
        let timespec: *const ffi::timespec = &ffi::timespec {
            tv_sec: delay.as_secs() as i64,
            tv_nsec: delay.as_nanos() as i32,
        };

        // Option<&T> is ABI compatible with *mut T.
        // *mut T is ABI compatible with *mut U
        // if <T as Pointee>::Metadata == <U as Pointee>::Metadata.
        // For all Sized types, <T as Pointee>::Metadata = ().
        let callback = unsafe { 
            mem::transmute::<
                extern "C" fn(alarm: u32, cb_arg: Option<&'a T>),
                extern "C" fn(alarm: u32, cb_arg: *mut c_void)
            >(callback)
        };
        let r = unsafe { ffi::SYS_SetAlarm(
            self.0, 
            timespec, 
            Some(callback), 
            cb_arg.map_or(ptr::null(), ptr::from_ref)
                .cast_mut()
                .cast()
        ) };

        assert!(r == 0, "Failed to set alarm - shouldn't be possible without ogc_sys raw usage");
    }

    /// Set the alarm parameters for a periodic alarm, add to the list of alarms and start.
    /// The alarm and interval persists as long as SYS_CancelAlarm() isn't called.
    /// 
    /// This function can softlock the console if called with a `start_delay` of 0 seconds
    /// on an alarm that's already set.
    pub fn set_periodic(
        &mut self,
        start_delay: Duration,
        period: Duration,
        callback: extern "C" fn(alarm: u32, cb_arg: Option<&'a T>),
        callback_data: impl Into<Option<&'a T>>,
    ) {
        let callback_data = callback_data.into();
        // Convert Duration to timespec
        let timespec_start: *const ffi::timespec = &ffi::timespec {
            tv_sec: start_delay.as_secs() as i64,
            tv_nsec: start_delay.as_nanos() as i32,
        };

        let timespec_period: *const ffi::timespec = &ffi::timespec {
            tv_sec: period.as_secs() as i64,
            tv_nsec: period.as_nanos() as i32,
        };
        // See set_alarm for safety explanation.
        let callback = unsafe { 
            mem::transmute::<
                extern "C" fn(alarm: u32, cb_arg: Option<&'a T>),
                extern "C" fn(alarm: u32, cb_arg: *mut c_void)
            >(callback)
        };
        let r = unsafe {
            ffi::SYS_SetPeriodicAlarm(
                self.0,
                timespec_start,
                timespec_period,
                Some(callback),
                callback_data.map_or(ptr::null(), ptr::from_ref)
                    .cast_mut()
                    .cast(),
            )
        };

        assert!(r == 0, "Failed to set periodic alarm - shouldn't be possible without ogc_sys raw usage");
    }
}

impl<T: Send> Drop for Alarm<'_, T> {
    fn drop(&mut self) {
        let r = unsafe { ffi::SYS_RemoveAlarm(self.0) };

        assert!(r == 0, "Failed to remove alarm - shouldn't be possible without ogc_sys raw usage");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord)]
pub struct Instant {
    pub secs: u64,
    pub nanos: u32,
}

impl Instant {
    pub fn now() -> Self {
        Self::from_raw(System::system_time_raw())
    }

    pub fn from_raw(raw: u64) -> Self {
        const TICKS_PER_SEC: u64 = ffi::TB_TIMER_CLOCK as u64 * 1000;
        let secs = raw / TICKS_PER_SEC;
        Self {
            secs,
            nanos: (((raw % TICKS_PER_SEC) * 4000) / (TB_TIMER_CLOCK as u64 / 250)) as u32
        }
    }
    pub fn elapsed(&self) -> Duration {
        Self::now().duration_since(*self)
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        if earlier.secs > self.secs || (earlier.secs == self.secs && earlier.nanos > self.nanos) {
            return Duration::new(0, 0);
        }
        let dur_s = self.secs - earlier.secs;
        if self.nanos < earlier.nanos {
            Duration::new(dur_s - 1, (self.nanos + 1_000_000_000) - earlier.nanos)
        } else {
            Duration::new(dur_s, self.nanos - earlier.nanos)
        }
    }
}

impl PartialOrd<Instant> for Instant {
    fn partial_cmp(&self, other: &Instant) -> Option<core::cmp::Ordering> {
        match self.secs.partial_cmp(&other.secs) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.nanos.partial_cmp(&other.nanos)
    }
}

/// OS Reset Types
#[derive(IntoPrimitive, TryFromPrimitive, Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ResetTypes {
    Restart = 0,
    HotReset = 1,
    Shutdown = 2,
    ReturnToMenu = 3,
    PowerOff = 4,
    PowerOffStandby = 5,
    PowerOffIdle = 6,
}

/// OS Memory Protection Modes
#[derive(IntoPrimitive, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MemoryProtectModes {
    ProtectNone = 0,
    ProtectRead = 1,
    ProtectWrite = 2,
    ProtectReadWrite = 1 | 2,
}

/// OS Memory Protection Channels
#[derive(IntoPrimitive, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MemoryProtectChannels {
    ChannelZero = 0,
    ChannelOne = 1,
    ChannelTwo = 2,
    ChannelThree = 3,
    All = 4,
}

/// System Font Header Structure
pub struct FontHeader {
    pub font_type: u16,
    pub first_char: u16,
    pub last_char: u16,
    pub inval_char: u16,
    pub asc: u16,
    pub desc: u16,
    pub width: u16,
    pub leading: u16,
    pub cell_dimensions: (u16, u16),
    pub sheet_size: u32,
    pub sheet_format: u16,
    pub sheet_colrow: (u16, u16),
    pub sheet_dimensions: (u16, u16),
    pub width_table: u16,
    pub sheet_image: u32,
    pub sheet_fullsize: u32,
}

impl From<&mut FontHeader> for *mut ffi::sys_fontheader {
    fn from(head: &mut FontHeader) -> *mut ffi::sys_fontheader {
        Box::into_raw(Box::new(ffi::sys_fontheader {
            font_type: head.font_type,
            first_char: head.first_char,
            last_char: head.last_char,
            inval_char: head.inval_char,
            asc: head.asc,
            desc: head.desc,
            width: head.width,
            leading: head.leading,
            cell_width: head.cell_dimensions.0,
            cell_height: head.cell_dimensions.1,
            sheet_size: head.sheet_size,
            sheet_format: head.sheet_format,
            sheet_column: head.sheet_colrow.0,
            sheet_row: head.sheet_colrow.1,
            sheet_width: head.sheet_dimensions.0,
            sheet_height: head.sheet_dimensions.1,
            width_table: head.width_table,
            sheet_image: head.sheet_image,
            sheet_fullsize: head.sheet_fullsize,
            c0: 0,
            c1: 0,
            c2: 0,
            c3: 0,
        }))
    }
}

/// Implementation of the system service.
impl System {
    /// Allocate cacheline aligned memory for the external
    /// framebuffer based on the rendermode object.
    ///
    /// This function returns a pointer to the framebuffer's startaddress which
    /// is aligned to a 32 byte boundary.
    pub fn allocate_framebuffer(render_mode: &RenderConfig) -> *mut c_void {
        unsafe { ffi::SYS_AllocateFramebuffer(render_mode.into()) }
    }

    /// Create and initialize sysalarm structure.
    pub fn create_alarm<'a, T: Send>() -> Result<Alarm<'a, T>> {
        let mut a = 0;
        let r = unsafe { ffi::SYS_CreateAlarm(&mut a) };

        if r < 0 {
            Err(OgcError::System("system failed to create alarm".into()))
        } else {
            Ok(Alarm(a, None, PhantomData))
        }
    }

    /// Init Font
    pub fn init_font(font_header: &mut FontHeader) {
        unsafe {
            let _ = ffi::SYS_InitFont(font_header.into());
        }
    }

    /// Get Font Texel
    ///
    /// # Safety
    ///
    /// The user must ensure the pointer is valid
    pub unsafe fn get_font_texel(
        c: i32,
        image: *mut c_void,
        position: i32,
        stride: i32,
        width: &mut i32,
    ) {
        ffi::SYS_GetFontTexel(c, image, position, stride, width);
    }

    /// Get Font Texture
    ///
    /// # Safety
    ///
    /// The user must ensure the pointer is valid
    pub unsafe fn get_font_texture(
        c: i32,
        image: *mut *mut c_void,
        xpos: &mut i32,
        ypos: &mut i32,
        width: &mut i32,
    ) {
        ffi::SYS_GetFontTexture(c, image, xpos, ypos, width);
    }

    /// Get Font Encoding
    pub fn get_font_encoding() -> u32 {
        unsafe { ffi::SYS_GetFontEncoding() }
    }

    /// Get Arena 1 Lo
    pub fn get_arena_1_lo() -> *mut c_void {
        unsafe { ffi::SYS_GetArena1Lo() }
    }

    /// Set Arena 1 Lo
    ///
    /// # Safety
    ///
    /// The user must ensure this point into the memory is valid and the arena doesn't go out
    /// memory
    pub unsafe fn set_arena_1_lo(new_lo: *mut c_void) {
        ffi::SYS_SetArena1Lo(new_lo)
    }

    /// Get Arena 1 Hi
    pub fn get_arena_1_hi() -> *mut c_void {
        unsafe { ffi::SYS_GetArena1Hi() }
    }

    /// Set Arena 1 Hi
    ///
    /// # Safety
    ///
    /// The user must ensure this point into the memory is valid and the arena doesn't go out
    /// memory
    pub unsafe fn set_arena_1_hi(new_hi: *mut c_void) {
        ffi::SYS_SetArena1Hi(new_hi)
    }

    /// Get Arena 1 Size
    pub fn get_arena_1_size() -> u32 {
        unsafe { ffi::SYS_GetArena1Size() }
    }

    /// Get Arena 2 Lo
    pub fn get_arena_2_lo() -> *mut c_void {
        unsafe { ffi::SYS_GetArena2Lo() }
    }

    /// Set Arena 2 Lo
    ///
    /// # Safety
    ///
    /// The user must ensure this point into the memory is valid and the arena doesn't go out
    /// memory
    pub unsafe fn set_arena_2_lo(new_lo: *mut c_void) {
        ffi::SYS_SetArena2Lo(new_lo)
    }

    /// Get Arena 2 Hi
    pub fn get_arena_2_hi() -> *mut c_void {
        unsafe { ffi::SYS_GetArena2Hi() }
    }

    /// Set Arena 2 Hi
    ///
    /// # Safety
    ///
    /// The user must ensure this point into the memory is valid and the arena doesn't go out
    /// memory
    pub unsafe fn set_arena_2_hi(new_hi: *mut c_void) {
        ffi::SYS_SetArena2Hi(new_hi)
    }

    /// Get Arena 2 Size
    pub fn get_arena_2_size() -> u32 {
        unsafe { ffi::SYS_GetArena2Size() }
    }

    /// Set Wireless ID
    pub fn set_wireless_id(channel: u32, id: u16) {
        unsafe {
            ffi::SYS_SetWirelessID(channel, id);
        }
    }

    /// Get Wireless ID
    pub fn get_wireless_id(channel: u32) -> u16 {
        unsafe { ffi::SYS_GetWirelessID(channel) }
    }

    /// Start PMC
    pub fn start_pmc(mcr0: u32, mcr1: u32) {
        unsafe {
            ffi::SYS_StartPMC(mcr0, mcr1);
        }
    }

    /// Dump PMC
    pub fn dump_pmc() {
        unsafe {
            ffi::SYS_DumpPMC();
        }
    }

    /// Stop PMC
    pub fn stop_pmc() {
        unsafe {
            ffi::SYS_StopPMC();
        }
    }

    /// Reset PMC
    pub fn reset_pmc() {
        unsafe {
            ffi::SYS_ResetPMC();
        }
    }

    /// Reset System
    pub fn reset_system(reset: ResetTypes, reset_type: u32, force_menu: i32) {
        unsafe {
            ffi::SYS_ResetSystem(reset.into(), reset_type, force_menu);
        }
    }

    /// Reset Button Down
    pub fn reset_button_down() -> u32 {
        unsafe { ffi::SYS_ResetButtonDown() }
    }

    /// Set Reset Callback
    ///
    /// Note: `ctx` is always null in the current version of libogc,
    /// but is still a required parameter.
    pub fn set_reset_callback(callback: extern "C" fn(irq: ResetTypes, ctx: *mut c_void)) {
        unsafe {
            // TODO: Do something with the returned callback.
            let callback = mem::transmute::<
                extern "C" fn(irq: ResetTypes, ctx: *mut c_void),
                extern "C" fn(irq: u32, ctx: *mut c_void),
            >(callback);
            let _ = ffi::SYS_SetResetCallback(Some(callback));
        }
    }

    /// Set Power Callback
    pub fn set_power_callback(callback: extern "C" fn()) {
        unsafe {
            // TODO: Do something with the returned callback.
            let _ = ffi::SYS_SetPowerCallback(Some(callback));
        }
    }

    /// Protect Range
    pub fn protect_range(
        channel: MemoryProtectChannels,
        // TODO: Why was this a u32? What is this used for?
        address: *const c_void,
        bytes: u32,
        control: MemoryProtectModes,
    ) {
        unsafe {
            ffi::SYS_ProtectRange(
                channel.into(),
                address as *mut c_void,
                bytes,
                control.into(),
            );
        }
    }

    /// Switch Fiber
    pub fn switch_fiber(arg0: u32, arg1: u32, arg2: u32, arg3: u32, pc: u32, newsp: u32) {
        unsafe {
            ffi::SYS_SwitchFiber(arg0, arg1, arg2, arg3, pc, newsp);
        }
    }

    /// Get Hollywood Revision
    pub fn get_hollywood_revision() -> u32 {
        unsafe { ffi::SYS_GetHollywoodRevision() }
    }

    /// Get system time in ticks.
    pub fn system_time_raw() -> u64 {
        unsafe { ffi::SYS_Time() }
    }

    /// Get system time.
    pub fn system_time() -> Instant {
        Instant::now()
    }

    /// Toggle reporting of stderr to the dolphin log.
    /// If `stdout` is `true`, also redirect stdout.
    /// 
    /// Calling this before Console::init will have no effect
    /// as it will be overridden to `false`.
    /// 
    /// OSREPORT Logging must be enabled in Dolphin
    /// for this to have any effect.
    pub fn stdio_report(stdout: bool) {
        unsafe { ffi::SYS_STDIO_Report(stdout); }
    }
}
