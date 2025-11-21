use libc::c_char;
use libc::c_uint;
use libc::c_void;
use std::ffi::CStr;
use std::{io, io::BufRead, thread};

#[used]
#[unsafe(link_section = "__DATA,__mod_init_func")]
static INIT: extern "C" fn() = {
    extern "C" fn init() {
        println!("!! LOADED AS A DYLIB !!");
        thread::spawn(|| {
            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                match line {
                    Ok(line) => {
                        println!("Activating godmode...");
                        start_driver();
                    }
                    Err(e) => eprintln!("Error getting input {:?}", e),
                }
            }
        });
    }
    init
};

// We will use some external C function from MacOS for the Dynamic Library
unsafe extern "C" {
    fn _dyld_image_count() -> c_uint;
    fn _dyld_get_image_header(index: c_uint) -> *const c_void;
    fn _dyld_get_image_name(index: c_uint) -> *const c_char;
}

fn start_driver() {
    let base_address = get_base().unwrap();
    let health_offset = vec![0x1d9ef0, 0x0, 0x418];

    unsafe {
        patch_h(health_offset, base_address);
    }
}

fn get_base() -> Option<usize> {
    /* In MacOS/IOS, we can't usually do LD_PRELOAD like functions since code is
    signed and expects a certain binary. We can either compile & self-sign, or remove SIP
    to get this going usually - but AssaultCube isn't signed!

    Using DYLD_INSERT_LIBRARIES, we can operate ahead of other libraries in DYLD_SHARE to
    do important stuff.

    Images here are considered binaries (exec or dyld itself) put into the process address space, so we might have:
    0 - BINARY EXECUTABLE
    1 - DYLD_SHARED_
    2..9999 - EXTRA DYLD, FRAMEWORK

    From testing, it seems that AssaultCube is always 0 by poking a sane and known deref.
    This can be done similar to walking an array of fun pointers! :paws:
    */
    let test: i32 = 0;
    let image_count = unsafe { _dyld_image_count() };
    // Get list of images
    let image_name = unsafe { CStr::from_ptr(_dyld_get_image_name(0)) };
    // unwrap
    let image_name_sanitised = image_name.to_str().unwrap_or("<unknown>");
    let image_header = unsafe { _dyld_get_image_header(0) };
    println!(
        "Image name {} found at address {:p} in Mach pages! Continuing...",
        image_name_sanitised, image_header
    );

    // Image header actually has the address to put ahead from now.
    Some(image_header as usize)
}

// This is the function that has HEAPS of derefs from pointers, so unsafe blocks ahoy!
// We can just run through the list - if we technically did skip to 0x0,0x418 prematurely,
// something went super wrong anyway.
unsafe fn patch_h(offsets: Vec<usize>, base_address: usize) {
    let mut addr = base_address;

    for i in 0..offsets.len() - 1 {
        // we can 'walk' the pointers here across the Player struct.
        // important to dereference each pointer to get an idea of what we are actually getting from
        // struct or not segfaulting on bad touch
        unsafe {
            addr = *(addr as *const usize).wrapping_add(offsets[i] / std::mem::size_of::<usize>());
        }
        if addr == 0 {
            println!("Null pointer while walking offset chain");
            return;
        }
    }

    let final_offset = offsets[offsets.len() - 1];
    let final_addr = (addr + final_offset) as *mut u64;

    println!("Final patch address: {:#x?}", final_addr);
    unsafe {
        println!("Health is at: {:#x?}", *final_addr);

        *final_addr = 9999;
        println!("And set to 9999...");
    }
}
