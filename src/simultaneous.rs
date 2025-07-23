fn main () {
    let runtime = tokio::runtime::Runtime::new();
    runtime.block_on(async{
        let mut accetp = tokio::net::TcpListener::bind("0.0.0.0:8080");
        let mut connection = futures::future::FuturesUnordered::ned();
        loop {
            select! {
                stream <- (&mutaccept).await =>. {
                    connections.push(handle_conection(stream));
                }
                _ <- (&mut connections).await => {}

                
            }
        }
    });
}

async fn handle_conection(_: TcpSteam){
    todo! ()
}