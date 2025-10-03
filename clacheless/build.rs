/*
    Copyright 2025 MydriaTech AB

    Licensed under the Apache License 2.0 with Free world makers exception
    1.0.0 (the "License"); you may not use this file except in compliance with
    the License. You should have obtained a copy of the License with the source
    or binary distribution in file named

        LICENSE-Apache-2.0-with-FWM-Exception-1.0.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

//! Custom build script for code generation.

/// Generate gRPC code from `proto/*.proto` files.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-changed=build.rs");
    // Get path to `protoc` executable bundled with `protoc_bin_vendored` crate
    let protoc_bin = protoc_bin_vendored::protoc_bin_path().unwrap();
    unsafe {
        std::env::set_var("PROTOC", &protoc_bin);
    }
    let dir = "proto".to_string();
    let proto_files = filenames_in_dir_by_ending(&dir, ".proto");
    //println!("warn:proto_files={proto_files:?}");
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&proto_files, &[dir])?;
    Ok(())
}

/// Return a list of relative filenames (including `dir`) that ends with
/// `ending`.
fn filenames_in_dir_by_ending(dir: &str, ending: &str) -> Vec<String> {
    std::fs::read_dir(std::path::PathBuf::from(dir))
        .ok()
        .map(|read_dir| {
            read_dir
                .filter_map(Result::ok)
                .by_ref()
                .map(|dir_entry| dir_entry.path())
                .filter_map(|path_buf| {
                    path_buf
                        .file_name()
                        .and_then(std::ffi::OsStr::to_str)
                        .map(str::to_string)
                })
                .filter(|filename| filename.ends_with(ending))
                .map(|filename| format!("{dir}{}{filename}", std::path::MAIN_SEPARATOR))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}
