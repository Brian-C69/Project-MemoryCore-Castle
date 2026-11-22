//! Fixed HTTP requests. Payload values never choose a URL, method, header or JSON key.
use super::*;
use serde::Deserialize;
use std::io::Read;
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub kind: String,
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub body: Value,
    #[serde(default)]
    pub header: String,
    #[serde(default)]
    pub key: String,
}
pub fn spec(v: &Value) -> Result<Spec, String> {
    let s: Spec = serde_json::from_value(v.clone()).map_err(|_| "Check the HTTP action fields.")?;
    if s.kind != "http" || !["GET", "POST"].contains(&s.method.as_str()) {
        return Err("Choose GET or POST.".into());
    }
    let url = reqwest::Url::parse(&s.url).map_err(|_| "Use a complete HTTP address.")?;
    let loopback = matches!(url.host_str(), Some("127.0.0.1" | "[::1]" | "::1"));
    if !(url.scheme() == "https" || (url.scheme() == "http" && loopback))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || s.url.contains(['{', '}', '\r', '\n'])
        || s.url.len() > 2048
    {
        return Err("Use HTTPS, or HTTP on a literal loopback address; no credentials or payload tokens in the address.".into());
    }
    if (s.key.is_empty() != s.header.is_empty())
        || (!s.key.is_empty() && s.key != "automation-http")
    {
        return Err("Choose the Automation HTTP key and its header together.".into());
    }
    if !s.header.is_empty() {
        let lower = s.header.to_ascii_lowercase();
        if reqwest::header::HeaderName::from_bytes(s.header.as_bytes()).is_err()
            || [
                "host",
                "content-length",
                "transfer-encoding",
                "connection",
                "cookie",
            ]
            .contains(&lower.as_str())
        {
            return Err("Choose a valid authentication header.".into());
        }
    }
    if s.body.to_string().len() > 32000 {
        return Err("Keep the JSON body under 32 KB.".into());
    }
    if s.method == "GET" && !s.body.is_null() {
        return Err("GET requests have no body.".into());
    }
    validate_body(&s.body)?;
    Ok(s)
}
fn validate_body(v: &Value) -> Result<(), String> {
    match v {
        Value::Object(o) => {
            for (k, v) in o {
                if k.contains("{{") {
                    return Err("Payload tokens cannot choose JSON keys.".into());
                }
                validate_body(v)?
            }
        }
        Value::Array(a) => {
            for v in a {
                validate_body(v)?
            }
        }
        Value::String(s) => {
            if s.replace("{{payload.text}}", "").contains("{{") {
                return Err("Only payload.text is supported in JSON string values.".into());
            }
        }
        _ => {}
    }
    Ok(())
}
pub fn body(v: &Value, payload: &Value) -> Value {
    match v {
        Value::String(s) => {
            Value::String(s.replace("{{payload.text}}", payload["text"].as_str().unwrap_or("")))
        }
        Value::Array(a) => Value::Array(a.iter().map(|v| body(v, payload)).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), body(v, payload)))
                .collect(),
        ),
        _ => v.clone(),
    }
}
fn bounded_body(v: &Value, payload: &Value) -> Result<Value, String> {
    fn check(v: &Value, text_len: usize, budget: &mut usize) -> Result<(), String> {
        let size = match v {
            Value::String(s) => {
                let count = s.matches("{{payload.text}}").count();
                s.len()
                    .checked_add(
                        count
                            .checked_mul(text_len)
                            .ok_or("Expanded JSON is too large.")?,
                    )
                    .ok_or("Expanded JSON is too large.")?
            }
            Value::Object(o) => {
                for (k, v) in o {
                    *budget = budget
                        .checked_sub(k.len())
                        .ok_or("Expanded JSON is too large.")?;
                    check(v, text_len, budget)?;
                }
                0
            }
            Value::Array(a) => {
                for v in a {
                    check(v, text_len, budget)?;
                }
                0
            }
            _ => 16,
        };
        *budget = budget
            .checked_sub(size)
            .ok_or("Expanded JSON is too large.")?;
        Ok(())
    }
    check(v, payload["text"].as_str().unwrap_or("").len(), &mut 65536)?;
    let result = body(v, payload);
    if result.to_string().len() > 65536 {
        return Err("Expanded JSON body exceeds64 KB.".into());
    }
    Ok(result)
}
pub fn run(
    s: &Spec,
    payload: &Value,
    timeout: std::time::Duration,
    cap: usize,
    key: Option<String>,
) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .map_err(|_| "The HTTP client could not start.")?;
    let mut req = if s.method == "POST" {
        client.post(&s.url).json(&bounded_body(&s.body, payload)?)
    } else {
        client.get(&s.url)
    };
    if !s.key.is_empty() {
        let value = key.ok_or("Save the Automation HTTP key in Settings first.")?;
        let value = reqwest::header::HeaderValue::from_str(&value)
            .map_err(|_| "The stored HTTP key is not a valid header value.")?;
        req = req.header(&s.header, value);
    }
    let response = req.send().map_err(|e| {
        if e.is_timeout() {
            "The HTTP request took too long."
        } else {
            "The HTTP request failed."
        }
    })?;
    let status = response.status();
    let mut bytes = Vec::new();
    response
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "The HTTP response could not be read.")?;
    if !status.is_success() {
        return Err(format!(
            "HTTP {}. Redirects are not followed.",
            status.as_u16()
        ));
    }
    let truncated = bytes.len() > cap;
    bytes.truncate(cap);
    Ok(format!(
        "HTTP {}\n{}{}",
        status.as_u16(),
        String::from_utf8_lossy(&bytes),
        if truncated {
            "\n[response trimmed]"
        } else {
            ""
        }
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn destinations_and_json_injection() {
        for url in [
            "http://example.com",
            "https://user:pw@example.com",
            "https://example.com/{{payload.text}}",
            "file:///x",
        ] {
            assert!(spec(&json!({"kind":"http","method":"GET","url":url})).is_err())
        }
        assert!(spec(&json!({"kind":"http","method":"GET","url":"http://127.0.0.1:9"})).is_ok());
        let p = json!({"text":"\"},\"evil\":true,\"x\":\"{{projects}}"});
        let b = body(&json!({"text":"{{payload.text}}"}), &p);
        assert_eq!(b.as_object().unwrap().len(), 1);
        assert_eq!(b["text"], p["text"]);
        assert!(spec(&json!({"kind":"http","method":"POST","url":"https://example.com","body":{"{{payload.text}}":"x"}})).is_err());
    }
    #[test]
    fn loopback_request_trim_and_redirect_refusal() {
        use std::io::Write;
        use std::net::TcpListener;
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.local_addr().unwrap());
        let t = std::thread::spawn(move || {
            let (mut socket, _) = server.accept().unwrap();
            let mut b = [0; 4096];
            socket.read(&mut b).unwrap();
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\n123456789",
                )
                .unwrap();
        });
        let s = spec(&json!({"kind":"http","method":"GET","url":url})).unwrap();
        assert!(
            run(&s, &json!({}), std::time::Duration::from_secs(2), 4, None)
                .unwrap()
                .contains("1234\n[response trimmed]")
        );
        t.join().unwrap();
    }
}
#[cfg(test)]
mod boundary_tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    #[test]
    fn post_body_escaping_authentication_and_no_redirect() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.local_addr().unwrap());
        let t = std::thread::spawn(move || {
            let (mut socket, _) = server.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut b = [0u8; 1024];
            loop {
                let n = socket.read(&mut b).unwrap();
                request.extend_from_slice(&b[..n]);
                if n == 0 {
                    break;
                }
                if let Some(at) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&request[..at]).to_lowercase();
                    let len = header
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length: "))
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    if request.len() >= at + 4 + len {
                        break;
                    }
                }
            }
            socket.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/should-not-follow\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
            request
        });
        let s=spec(&json!({"kind":"http","method":"POST","url":url,"header":"X-Fixture-Key","key":"automation-http","body":{"text":"{{payload.text}}"}})).unwrap();
        let malicious = "\"},\"injected\":true,\"x\":\"";
        assert!(run(
            &s,
            &json!({"text":malicious}),
            std::time::Duration::from_secs(2),
            1024,
            Some("fixture-only".into())
        )
        .unwrap_err()
        .contains("302"));
        let request = t.join().unwrap();
        let at = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        assert!(String::from_utf8_lossy(&request[..at])
            .to_lowercase()
            .contains("x-fixture-key: fixture-only"));
        let value: Value = serde_json::from_slice(&request[at + 4..]).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 1);
        assert_eq!(value["text"], malicious);
    }
    #[test]
    fn header_and_method_gates() {
        for action in [
            json!({"kind":"http","method":"DELETE","url":"https://example.com"}),
            json!({"kind":"http","method":"GET","url":"https://example.com","header":"Host","key":"automation-http"}),
            json!({"kind":"http","method":"GET","url":"https://example.com","header":"Authorization","key":"openai"}),
            json!({"kind":"http","method":"GET","url":"https://example.com","body":{}}),
        ] {
            assert!(spec(&action).is_err());
        }
    }
}
#[cfg(test)]mod expanded_tests{use super::*;#[test]fn expanded_body_is_bounded_before_request(){let template=json!({"text":"{{payload.text}}".repeat(100)});assert!(bounded_body(&template,&json!({"text":"x".repeat(16384)})).is_err());assert_eq!(bounded_body(&json!({"text":"{{payload.text}}"}),&json!({"text":"{{payload.text}}"})).unwrap()["text"],"{{payload.text}}");}}
