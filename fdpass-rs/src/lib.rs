// Copyright (C) 2014-2016 Mickaël Salaün
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Lesser General Public License as published by
// the Free Software Foundation, version 3 of the License.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

extern crate async_std;
extern crate fd;
extern crate libc;

use async_std::os::unix::net::UnixStream as asyncUnixStream;
use fd::FileDesc;
use ffi::{recvmsg, sendmsg, Cmsghdr, Iovec, Msghdr, Scm, SOL_SOCKET};
use libc::{c_void, size_t};
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream as stdUnixStream;

mod ffi;

#[repr(C)]
struct FdPadding {
    pub fd: RawFd,
    /* __cmsg_data must be align with size_t */
    #[cfg(target_arch = "x86_64")]
    _padding: u32,
}

impl FdPadding {
    #[cfg(target_arch = "x86_64")]
    pub fn new(fd: RawFd) -> FdPadding {
        FdPadding {
            fd: fd,
            _padding: 0,
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    pub fn new(fd: RawFd) -> FdPadding {
        FdPadding { fd: fd }
    }
}

pub fn recv_fd(stream: &mut stdUnixStream, iov_expect: Vec<u8>) -> io::Result<FileDesc> {
    let fd = FdPadding::new(-1 as RawFd);
    match recvmsg(stream, iov_expect.len(), fd) {
        // TODO: Check size?
        Ok((_, iov_recv, data)) => {
            if iov_recv != iov_expect {
                return Err(io::Error::new(io::ErrorKind::Other, "Receive"));
            }
            Ok(FileDesc::new(data.fd, true))
        }
        Err(e) => Err(e),
    }
}

pub fn async_recv_fd(stream: &mut asyncUnixStream, iov_expect: Vec<u8>) -> io::Result<FileDesc> {
    let fd = FdPadding::new(-1 as RawFd);
    match recvmsg(stream, iov_expect.len(), fd) {
        // TODO: Check size?
        Ok((_, iov_recv, data)) => {
            if iov_recv != iov_expect {
                return Err(io::Error::new(io::ErrorKind::Other, "Receive"));
            }
            Ok(FileDesc::new(data.fd, true))
        }
        Err(e) => Err(e),
    }
}

pub fn send_fd(stream: &mut stdUnixStream, id: &[u8], fd: &dyn AsRawFd) -> io::Result<()> {
    let mut iovv = vec![Iovec {
        iov_base: id.as_ptr() as *const c_void,
        iov_len: id.len() as size_t,
    }];
    let fda = FdPadding::new(fd.as_raw_fd());
    let mut ctrl = Cmsghdr::new(SOL_SOCKET, Scm::Rights, fda);
    let msg = Msghdr::new(None, &mut iovv, &mut ctrl, None);
    match sendmsg(stream, &msg) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn async_send_fd(stream: &mut asyncUnixStream, id: &[u8], fd: &dyn AsRawFd) -> io::Result<()> {
    let mut iovv = vec![Iovec {
        iov_base: id.as_ptr() as *const c_void,
        iov_len: id.len() as size_t,
    }];
    let fda = FdPadding::new(fd.as_raw_fd());
    let mut ctrl = Cmsghdr::new(SOL_SOCKET, Scm::Rights, fda);
    let msg = Msghdr::new(None, &mut iovv, &mut ctrl, None);
    match sendmsg(stream, &msg) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}