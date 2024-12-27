// use super::typedefs::block_info::BlockInfo;
// use async_stream::stream;
// use futures::{pin_mut, Stream, StreamExt};
// use solana_client::nonblocking::rpc_client::RpcClient;
// use std::sync::Arc;
// pub mod grpc;
// pub mod poller;
// use grpc::get_grpc_stream_with_rpc_fallback;
// use poller::get_block_poller_stream;

// pub struct BlockStreamConfig {
//     pub rpc_client: Arc<RpcClient>,
//     pub geyser_url: Option<String>,
//     pub max_concurrent_block_fetches: usize,
//     pub last_indexed_slot: u64,
// }

// impl BlockStreamConfig {
//     pub fn load_block_stream(&self) -> impl Stream<Item = Vec<BlockInfo>> {
//         let grpc_stream = self.geyser_url.as_ref().map(|geyser_url| {
//             let auth_header = std::env::var("GRPC_X_TOKEN").unwrap();
//             get_grpc_stream_with_rpc_fallback(
//                 geyser_url.clone(),
//                 auth_header,
//                 self.rpc_client.clone(),
//                 self.last_indexed_slot,
//                 self.max_concurrent_block_fetches,
//             )
//         });

//         let poller_stream = if self.geyser_url.is_none() {
//             Some(get_block_poller_stream(
//                 self.rpc_client.clone(),
//                 self.last_indexed_slot,
//                 self.max_concurrent_block_fetches,
//             ))
//         } else {
//             None
//         };

//         stream! {
//             loop {
//                 // Check and yield from the gRPC stream if available
//                 if let Some(grpc_stream) = grpc_stream.as_mut() {
//                     pin_mut!(grpc_stream);
//                     while let Some(blocks) = grpc_stream.next().await {
//                         yield blocks;
//                     }
//                 }

//                 // If gRPC stream is not available, use the poller stream
//                 if let Some(poller_stream) = poller_stream.as_mut() {
//                     pin_mut!(poller_stream);
//                     while let Some(blocks) = poller_stream.next().await {
//                         yield blocks;
//                     }
//                 }

//                 // Optional: Add a small delay to prevent busy-waiting
//                 tokio::time::sleep(std::time::Duration::from_millis(100)).await;
//             }
//         }
//     }
// }
