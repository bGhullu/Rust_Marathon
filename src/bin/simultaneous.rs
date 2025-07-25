pub fn simultaneous () {
    let runtime = tokio::runtime::Runtime::new();
    runtime.block_on(async{
        let mut accetp = tokio::net::TcpListener::bind("0.0.0.0:8080");
        while let Ok(stream) = accept.wait {
            tokio::spawn(handle_conection(stream));
        }
    });
}

async fn handle_conection(_: TcpSteam){
    tokio::spawn(async{

    })
}