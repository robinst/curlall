use curlall::{Opt, NAME};
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Request, Response, Server, StatusCode};
use std::fmt::Write;
use std::process::Output;
use std::time::Duration;
use tokio::process::Command;

// Basic auth for "admin:hunter2"
const BASIC_AUTH: &str = "Basic YWRtaW46aHVudGVyMg==";

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

fn json_response(s: &'static str) -> Result<Response<Body>> {
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(s))?)
}

async fn generic(request: Request<Body>) -> Result<Response<Body>> {
    let path_and_query = request.uri().path_and_query().unwrap().as_str();
    let auth = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    match path_and_query {
        "/without-link" => json_response(
            r#"{
                "values": [1, 2]
            }"#,
        ),
        "/without-link?page=2" => json_response(
            r#"{
                "values": [3, 4]
            }"#,
        ),
        "/without-link?page=3" => json_response(
            r#"{
                "values": [5, 6]
            }"#,
        ),
        "/without-link?page=4" => json_response(
            r#"{
                "values": []
            }"#,
        ),
        "/basic-auth" if auth == BASIC_AUTH => json_response(
            r#"{
                "values": [1, 2]
            }"#,
        ),
        "/basic-auth?page=2" if auth == BASIC_AUTH => json_response(
            r#"{
                "values": [3, 4]
            }"#,
        ),
        "/non-numeric-page?page=x" => json_response(
            r#"{
                "values": [1, 2]
            }"#,
        ),
        "/echo-headers" => {
            let mut formatted = String::new();
            for (key, value) in request.headers().iter() {
                write!(&mut formatted, "{}: {}, ", key, value.to_str()?)?;
            }
            Ok(Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{
                        "values": ["{}"]
                    }}"#,
                    formatted
                )))?)
        }
        "/error-500" => Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::empty())?),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())?),
    }
}

async fn bitbucket(request: Request<Body>) -> Result<Response<Body>> {
    let path_and_query = request.uri().path_and_query().unwrap().as_str();
    match path_and_query {
        "/next-link" => json_response(
            r#"{
                "values": [1, 2],
                "next": "/next-link?page=b"
            }"#,
        ),
        "/next-link?page=b" => json_response(
            r#"{
                "values": [3, 4],
                "next": "/next-link?page=c"
            }"#,
        ),
        "/next-link?page=c" => json_response(
            r#"{
                "values": [5]
            }"#,
        ),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())?),
    }
}

async fn github(request: Request<Body>) -> Result<Response<Body>> {
    let path_and_query = request.uri().path_and_query().unwrap().as_str();
    match path_and_query {
        "/link-header" => Ok(Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::LINK, "</link-header?page=b>; rel=\"next\"")
            .body(Body::from(r#"{"items": [1, 2]}"#))?),
        "/link-header?page=b" => Ok(Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .header(
                header::LINK,
                "</link-header?page=c>; rel=\"next\", </link-header>; rel=\"prev\"",
            )
            .body(Body::from(r#"{"items": [3, 4]}"#))?),
        "/link-header?page=c" => Ok(Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::LINK, "</link-header?page=b>; rel=\"prev\"")
            .body(Body::from(r#"{"items": [5]}"#))?),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())?),
    }
}

async fn run_success_url(url: String) -> String {
    run_success(Opt {
        url,
        ..Opt::default()
    })
    .await
}

async fn run_success(opt: Opt) -> String {
    let output = run(opt).await;
    let success = output.status.success();
    if !success {
        assert!(
            success,
            "Command failed unexpectedly: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}

async fn run_error(opt: Opt) -> String {
    let output = run(opt).await;
    let success = output.status.success();
    if success {
        assert!(
            !success,
            "Command expected to error, but was successful: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    String::from_utf8_lossy(&output.stderr).to_string()
}

async fn run(opt: Opt) -> Output {
    // Rust 1.43.0 added CARGO_BIN_EXE_, fall back to manual path if not available.
    let path = option_env!("CARGO_BIN_EXE_curlall").unwrap_or("./target/debug/curlall");
    let mut command = Command::new(path);
    if let Some(limit) = opt.limit {
        command.arg("--limit").arg(format!("{}", limit));
    }
    if let Some(wait) = opt.wait {
        command.arg("--wait").arg(format!("{}", wait.as_secs_f64()));
    }
    if let Some(user_password) = opt.user_password {
        command.arg("--user").arg(user_password);
    }
    for header in &opt.headers {
        command.arg("--header").arg(header);
    }
    command.arg(opt.url);
    command.output().await.expect("failed to run command")
}

#[tokio::test]
async fn test_generic() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let make_svc = make_service_fn(|_conn| async { Ok::<_, GenericError>(service_fn(generic)) });
    let server = Server::bind(&([127, 0, 0, 1], 0).into()).serve(make_svc);
    let base_url = format!("http://{}", server.local_addr());
    tokio::spawn(server);

    let stdout = run_success_url(format!("{}/without-link", base_url)).await;
    assert_eq!(stdout, "1\n2\n3\n4\n5\n6\n");

    let stdout = run_success_url(format!("{}/without-link?page=2", base_url)).await;
    assert_eq!(stdout, "3\n4\n5\n6\n");

    let stdout = run_success(Opt {
        url: format!("{}/without-link", base_url),
        limit: Some(1),
        ..Opt::default()
    })
    .await;
    assert_eq!(stdout, "1\n");

    let stdout = run_success(Opt {
        url: format!("{}/without-link", base_url),
        limit: Some(2),
        ..Opt::default()
    })
    .await;
    assert_eq!(stdout, "1\n2\n");

    let stdout = run_success(Opt {
        url: format!("{}/without-link", base_url),
        wait: Some(Duration::from_secs_f64(0.1)),
        ..Opt::default()
    })
    .await;
    assert_eq!(stdout, "1\n2\n3\n4\n5\n6\n");

    let stdout = run_success(Opt {
        url: format!("{}/basic-auth", base_url),
        user_password: Some("admin:hunter2".to_string()),
        ..Opt::default()
    })
    .await;
    assert_eq!(stdout, "1\n2\n3\n4\n");

    let stdout = run_success(Opt {
        url: format!("{}/basic-auth", base_url),
        headers: vec![format!("Authorization: {}", BASIC_AUTH)],
        ..Opt::default()
    })
    .await;
    assert_eq!(stdout, "1\n2\n3\n4\n");

    let stdout = run_success(Opt {
        url: format!("{}/echo-headers", base_url),
        headers: vec![
            "Foo: 1".to_string(),
            "Foo: 2".to_string(),
            "Bar: a: b".to_string(),
        ],
        ..Opt::default()
    })
    .await;
    assert!(stdout.contains(r#"foo: 1"#), stdout);
    assert!(stdout.contains(r#"foo: 2"#));
    assert!(stdout.contains(r#"bar: a: b"#));

    let stderr = run_error(Opt {
        url: format!("{}/non-numeric-page?page=x", base_url),
        ..Opt::default()
    })
    .await;
    assert!(stderr.contains("Page query param 'x' could not be parsed as a number"));

    let stderr = run_error(Opt {
        url: format!("{}/error-500", base_url),
        ..Opt::default()
    })
    .await;
    let expected = format!(
        "{}: Error getting {}/error-500: 500 Internal Server Error: \n",
        NAME, base_url
    );
    assert_eq!(stderr, expected);

    Ok(())
}

#[tokio::test]
async fn test_bitbucket() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let make_svc = make_service_fn(|_conn| async { Ok::<_, GenericError>(service_fn(bitbucket)) });
    let server = Server::bind(&([127, 0, 0, 1], 0).into()).serve(make_svc);
    let base_url = format!("http://{}", server.local_addr());
    tokio::spawn(server);

    let stdout = run_success_url(format!("{}/next-link", base_url)).await;
    assert_eq!(stdout, "1\n2\n3\n4\n5\n");

    let stdout = run_success_url(format!("{}/next-link?page=b", base_url)).await;
    assert_eq!(stdout, "3\n4\n5\n");

    Ok(())
}

#[tokio::test]
async fn test_github() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let make_svc = make_service_fn(|_conn| async { Ok::<_, GenericError>(service_fn(github)) });
    let server = Server::bind(&([127, 0, 0, 1], 0).into()).serve(make_svc);
    let base_url = format!("http://{}", server.local_addr());
    tokio::spawn(server);

    let stdout = run_success_url(format!("{}/link-header", base_url)).await;
    assert_eq!(stdout, "1\n2\n3\n4\n5\n");

    let stdout = run_success_url(format!("{}/link-header?page=b", base_url)).await;
    assert_eq!(stdout, "3\n4\n5\n");

    Ok(())
}
