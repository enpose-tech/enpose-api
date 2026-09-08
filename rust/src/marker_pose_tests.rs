use super::MarkerPose;

fn sample(marker_id: u16) -> MarkerPose {
    MarkerPose {
        timestamp: 0x0102_0304_0506_0708,
        marker_id,
        x: 1.5,
        y: -2.25,
        z: 3.125,
        rotation: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        position_rmse: 0.1,
        rotation_rmse: 0.01,
        sensors: 3,
        observed_emitters: 4,
    }
}

#[test]
fn round_trips_a_batch() {
    let poses = vec![sample(7), sample(8), sample(9)];
    let encoded = MarkerPose::encode_batch(&poses);
    // 2-byte count + N fixed-size records.
    assert_eq!(encoded.len(), 2 + poses.len() * MarkerPose::ENCODED_SIZE);
    assert_eq!(MarkerPose::decode_batch(&encoded), Some(poses));
}

#[test]
fn round_trips_an_empty_batch() {
    let encoded = MarkerPose::encode_batch(&[]);
    assert_eq!(encoded, vec![0, 0]);
    assert_eq!(MarkerPose::decode_batch(&encoded), Some(Vec::new()));
}

#[test]
fn encodes_big_endian() {
    let encoded = MarkerPose::encode_batch(&[sample(0xBEEF)]);
    // count = 1, then timestamp big-endian, then marker_id big-endian.
    assert_eq!(&encoded[0..2], &[0x00, 0x01]);
    assert_eq!(&encoded[2..10], &0x0102_0304_0506_0708u64.to_be_bytes());
    assert_eq!(&encoded[10..12], &[0xBE, 0xEF]);
}

#[test]
fn rejects_truncated_payloads() {
    let encoded = MarkerPose::encode_batch(&[sample(1)]);
    // Too short for the declared count of 1.
    assert_eq!(MarkerPose::decode_batch(&encoded[..encoded.len() - 1]), None);
    // A buffer too short even for the count prefix.
    assert_eq!(MarkerPose::decode_batch(&[0]), None);
    // Trailing garbage past the declared count is also rejected.
    let mut extra = encoded.clone();
    extra.push(0xFF);
    assert_eq!(MarkerPose::decode_batch(&extra), None);
}
