// use std::{
//     fs,
//     os::unix::{io::FromRawFd, net::UnixStream},
//     io::Read,
//     // ffi::CString,
// };

use async_std::{
    // fs,
    os::unix::{io::FromRawFd, net::UnixStream},
    // io::Read,
    // ffi::CString,
    task,
};
use libc::{ftruncate, mmap, MAP_SHARED, PROT_READ, PROT_WRITE};

use ringbuf::RingBuffer;
use std::{
    collections::VecDeque,
    fs,
    io::{IoSliceMut, Read},
    ptr::null_mut,
};
// use nix::unistd::ftruncate;

// const BUF_SIZE: i64 = 1536 * 20;

async fn async_main() {
    let mut stream = UnixStream::connect("/tmp/async.sock").await.unwrap();
    let shm_fd = shm_open_anonymous::shm_open_anonymous();
    // ftruncate(shm_fd, BUF_SIZE).unwrap();
    println!("Shm fd: {}", shm_fd);
    if let Err(e) = fdpass::async_send_fd(&mut stream, &[0], &shm_fd) {
        println!("Errored out: {:#?}", e);
    }
    unsafe { ftruncate(shm_fd, 1536 * 20 * 2) };
    println!("done");
    let v_ptr = unsafe {
        mmap(
            null_mut(),
            1536 * 20 * 2,
            PROT_READ | PROT_WRITE,
            MAP_SHARED,
            shm_fd,
            0,
        )
    };
    println!("mmap completed");
    let r: VecDeque<u8> = VecDeque::with_capacity(1536 * 20);
    let r_buf = unsafe { &mut *(v_ptr as *mut VecDeque<u8>) };
    unsafe { std::ptr::write(r_buf, r) };
    println!("read buf cap: {}", r_buf.capacity());
    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("read  elem: {:#?}", r_buf.pop_back());
    let _v_ptr = unsafe { v_ptr.offset(1536 * 20) };
    let w_buf = unsafe { &mut *(_v_ptr as *mut RingBuffer<[u8; 1536]>) };
    println!("read buf cap: {}", w_buf.capacity());
    // let mut file = unsafe { fs::File::from_raw_fd(shm_fd) };
    loop {
        // let mut b = [0; 1536];
        // let mut buf = [IoSliceMut::new(&mut b)];
        // match file.read_vectored(&mut buf) {
        //     Ok(size) => {
        //         if size > 0 {
        //             println!("read {} bytes", size);
        //             // println!("Data: {}", buf);
        //             for c in buf.chunks(1536) {
        //                 println!("Data: {}", str::from_utf8(&c[0]).unwrap());
        //             }
        //         }
        //     }
        //     Err(_e) => {}
        // }
    }
}

// use nix::{unistd::ftruncate, sys::memfd::{self, memfd_create}};

// const BUF_SIZE: i64 = 1536 * 20;

fn main() {
    // let mut stream = UnixStream::connect("/tmp/async.sock").unwrap();
    // let shm_fd = shm_open_anonymous::shm_open_anonymous();
    // // let cstr = CString::new("ANON").unwrap();
    // // let shm_fd = memfd_create(&cstr, memfd::MemFdCreateFlag::MFD_ALLOW_SEALING).unwrap();
    // // let _ = ftruncate(shm_fd, BUF_SIZE);
    // println!("Shm fd: {}", shm_fd);
    // if let Err(e) = fdpass::send_fd(&mut stream, &[0], &shm_fd) {
    //     println!("Errored out: {:#?}", e);
    // }
    // // stream.write_all(b"hello").unwrap();
    // println!("done");
    // let mut file = unsafe { fs::File::from_raw_fd(shm_fd) };
    // loop {
    //     let mut buf = String::new();
    //     match file.read_to_string(&mut buf) {
    //         Ok(size) => {
    //             if size > 0 {
    //                 println!("read {} bytes", size);
    //                 println!("Data: {}", buf);
    //             }
    //         },
    //         Err(_e) => {},
    //         }
    // }
    task::block_on(async_main());
}
