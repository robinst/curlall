use curlall::{run_async, Opt};
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Request, Response, Server};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

async fn empty(_: Request<Body>) -> Result<Response<Body>> {
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"values": []}"#))?)
}

#[tokio::test]
async fn test_empty() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let make_svc = make_service_fn(|_conn| async { Ok::<_, GenericError>(service_fn(empty)) });

    let server = Server::bind(&([127, 0, 0, 1], 0).into()).serve(make_svc);
    let base_url = format!("http://{}", server.local_addr());
    tokio::spawn(server);

    let opt = Opt {
        user_password: Some("user:password".to_string()),
        number: Some(10),
        url: format!("{}/foo", base_url),
    };

    run_async(opt).await?;
    Ok(())
}
