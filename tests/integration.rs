mod environment;

use aws_sdk_s3::config::http::HttpResponse;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::list_buckets::{ListBucketsError, ListBucketsOutput};
use crate::environment::Environment;

#[tokio::test]
async fn test_s3local_stack_starts() {
    let env = Environment::setup().await;
    //
    let result: Result<ListBucketsOutput, SdkError<ListBucketsError, HttpResponse>> = env.aws_client.list_buckets().send().await;
    assert!(result.is_ok());

    if let Some(buckets) = result.unwrap().buckets {
        let bucket_name = buckets[0].name.clone().unwrap();
        assert_eq!(bucket_name, "test-bucket");
    }
}