use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const CONNECT_RETRY_DELAY: Duration = Duration::from_micros(50);
const INSTANCE_START: &str = r#"{"action_type":"InstanceStart"}"#;

fn main() -> Result<()> {
    let mut args = env::args_os().skip(1);
    let (Some(socket), Some(config_path), None) = (args.next(), args.next(), args.next()) else {
        bail!("usage: fc-api <API_SOCKET> <VM_CONFIG_JSON>");
    };
    let config_path = Path::new(&config_path);
    let config = fs::read_to_string(config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let requests = config_requests(&config)?;

    let stream = connect(Path::new(&socket), CONNECT_TIMEOUT)?;

    let mut reader = BufReader::new(&stream);
    let mut writer = &stream;
    for (resource, body) in &requests {
        put(&mut reader, &mut writer, resource, body)?;
    }
    put(&mut reader, &mut writer, "actions", INSTANCE_START)
}

fn config_requests(config: &str) -> Result<Vec<(String, String)>> {
    let Value::Object(sections) = serde_json::from_str(config).context("parsing the VM config")?
    else {
        bail!("the VM config must be a JSON object");
    };
    let mut requests = Vec::with_capacity(sections.len());
    for (name, value) in sections {
        match value {
            Value::Object(_) => requests.push((name, value.to_string())),
            Value::Array(items) if items.is_empty() => {}
            _ => bail!("unsupported section {name:?}: only objects and empty arrays are"),
        }
    }
    Ok(requests)
}

fn connect(socket: &Path, timeout: Duration) -> Result<UnixStream> {
    let deadline = Instant::now() + timeout;
    loop {
        match UnixStream::connect(socket) {
            Ok(stream) => return Ok(stream),
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) && Instant::now() < deadline =>
            {
                thread::sleep(CONNECT_RETRY_DELAY);
            }
            Err(err) => {
                return Err(err).with_context(|| format!("connecting to {}", socket.display()))
            }
        }
    }
}

fn put(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    resource: &str,
    body: &str,
) -> Result<()> {
    let request = format!(
        "PUT /{resource} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\n\r\n{body}",
        body.len()
    );
    writer.write_all(request.as_bytes())?;
    let (status, response) = read_response(reader)?;
    ensure!(
        (200..300).contains(&status),
        "PUT /{resource} failed with HTTP {status}: {response}"
    );
    Ok(())
}

fn read_response(reader: &mut impl BufRead) -> Result<(u16, String)> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let status = line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .with_context(|| format!("invalid HTTP status line {line:?}"))?;

    let mut content_length = 0;
    loop {
        line.clear();
        ensure!(
            reader.read_line(&mut line)? > 0,
            "connection closed in HTTP headers"
        );
        let header = line.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().context("invalid Content-Length")?;
            }
        }
    }

    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;
    Ok((status, String::from_utf8_lossy(&body).into_owned()))
}