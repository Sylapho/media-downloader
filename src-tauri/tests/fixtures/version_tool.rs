use std::{
    env,
    io::{self, Write},
    process, thread,
    time::Duration,
};

fn main() {
    let executable = env::current_exe().unwrap();
    let name = executable.file_stem().unwrap().to_str().unwrap();
    let args: Vec<_> = env::args().skip(1).collect();
    match name {
        "metadata" => {
            let boundary = args
                .iter()
                .position(|arg| arg == "--")
                .expect("URL terminator required");
            assert_eq!(args.len(), boundary + 2);
            assert_eq!(
                args[boundary + 1],
                "https://media.example.org/watch?v=x&text=hello;%24(calc)"
            );
            for flag in [
                "--ignore-config",
                "--no-playlist",
                "--dump-single-json",
                "--simulate",
                "--skip-download",
                "--no-cache-dir",
            ] {
                assert!(args[..boundary].iter().any(|arg| arg == flag));
            }
            let fixture = include_str!("media.json").trim().strip_suffix('}').unwrap();
            println!("{fixture},\"padding\":\"{}\"}}", "x".repeat(20 * 1024));
        }
        "yt-dlp" => {
            assert_eq!(args, ["--version"]);
            println!("2026.10.01");
        }
        "ffmpeg" | "ffprobe" => {
            assert_eq!(args, ["-version"]);
            println!("{name} version 8.0.1-essentials_build");
        }
        "failed" => {
            eprintln!("deliberate failure");
            process::exit(2);
        }
        "flood" => {
            io::stdout().write_all(&vec![b'x'; 100_000]).unwrap();
        }
        "stderr-flood" => {
            io::stderr().write_all(&vec![b'x'; 100_000]).unwrap();
        }
        "slow" | "cancelled" => {
            std::fs::write(
                executable.parent().unwrap().join(format!("{name}-started")),
                "started",
            )
            .unwrap();
            thread::sleep(Duration::from_secs(2));
            std::fs::write(
                executable
                    .parent()
                    .unwrap()
                    .join(format!("{name}-completed")),
                "survived",
            )
            .unwrap();
        }
        _ => process::exit(3),
    }
}
