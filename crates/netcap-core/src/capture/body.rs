use bytes::Bytes;
use flate2::read::{DeflateDecoder, GzDecoder};
use std::io::Read;

pub fn truncate_body(body: &Bytes, max_size: usize) -> (Bytes, bool) {
    if body.len() <= max_size {
        (body.clone(), false)
    } else {
        (body.slice(..max_size), true)
    }
}

pub fn decode_body(body: &[u8], encoding: &str) -> Result<Vec<u8>, std::io::Error> {
    match encoding {
        "gzip" => {
            let mut decoder = GzDecoder::new(body);
            let mut decoded = Vec::new();
            decoder.read_to_end(&mut decoded)?;
            Ok(decoded)
        }
        "deflate" => {
            let mut decoder = DeflateDecoder::new(body);
            let mut decoded = Vec::new();
            decoder.read_to_end(&mut decoded)?;
            Ok(decoded)
        }
        "br" => {
            let mut decoded = Vec::new();
            brotli::BrotliDecompress(&mut &body[..], &mut decoded)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(decoded)
        }
        _ => Ok(body.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    #[test]
    fn truncate_body_small() {
        let body = Bytes::from("hello");
        let (result, truncated) = truncate_body(&body, 100);
        assert_eq!(result, body);
        assert!(!truncated);
    }

    #[test]
    fn truncate_body_exact() {
        let body = Bytes::from("hello");
        let (result, truncated) = truncate_body(&body, 5);
        assert_eq!(result, body);
        assert!(!truncated);
    }

    #[test]
    fn truncate_body_large() {
        let body = Bytes::from("hello world");
        let (result, truncated) = truncate_body(&body, 5);
        assert_eq!(result, Bytes::from("hello"));
        assert!(truncated);
    }

    #[test]
    fn decode_gzip() {
        let original = b"hello world";
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decoded = decode_body(&compressed, "gzip").unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_identity() {
        let data = b"plain text";
        let decoded = decode_body(data, "identity").unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn decode_unknown_encoding() {
        let data = b"some data";
        let decoded = decode_body(data, "unknown-encoding").unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn decode_invalid_gzip() {
        let data = b"not gzip data";
        let result = decode_body(data, "gzip");
        assert!(result.is_err());
    }

    #[test]
    fn decode_deflate() {
        use flate2::write::DeflateEncoder;
        let original = b"deflate test data";
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decoded = decode_body(&compressed, "deflate").unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_brotli() {
        let original = b"brotli test data for compression";
        let mut compressed = Vec::new();
        {
            let mut writer =
                brotli::CompressorWriter::new(&mut compressed, 4096, 6, 22);
            writer.write_all(original).unwrap();
        }
        let decoded = decode_body(&compressed, "br").unwrap();
        assert_eq!(decoded, original);
    }
}
