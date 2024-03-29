// Copyright (C) 2016 Mickaël Salaün
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

extern crate fdpass;

use fdpass;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::process::exit;
use std::os::unix::net::UnixStream;

static SOCKET_PATH: &'static str = "./tmp_fdpass.sock";


fn recv_and_print(socket_path: &str) -> io::Result<()> {
    let mut stream = UnixStream::connect(socket_path).unwrap();
    let fd = fdpass::recv_fd(&mut stream, vec!(0u8)).unwrap();
    println!("Received {:?}", fd);
    let mut file = unsafe { File::from_raw_fd(fd.into_raw_fd()) };
    let _ = file.seek(SeekFrom::Start(0)).unwrap();
    let mut buffer = String::new();
    let size = file.read_to_string(&mut buffer).unwrap();
    println!("Read {} bytes: {}", size, buffer);
    Ok(())
}

fn main() {
    if let Err(e) = recv_and_print(SOCKET_PATH) {
        println!("Error: {}", e);
        exit(1);
    }
}
