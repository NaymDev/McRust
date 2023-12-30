/*use tokio::{io::{AsyncReadExt, self}, net::tcp::OwnedReadHalf};

pub async fn read_varint(stream: &mut OwnedReadHalf) -> io::Result<u64> {
    let mut result = 0;
    let mut shift = 0;

    loop {
        let byte = stream.read_u8().await?;
        result |= ((byte & 0x7F) as u64) << shift;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
    }

    Ok(result)
}*/
use tokio::{io::{self, AsyncReadExt}, net::tcp::OwnedReadHalf};

pub async fn read_varint(stream: &mut OwnedReadHalf) -> io::Result<u64> {
    let mut result = 0;
    let mut shift = 0;

    loop {
        let byte = match stream.read_u8().await {
            Ok(b) => b,
            Err(err) => {
                if err.kind() == io::ErrorKind::UnexpectedEof {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Unexpected end of stream"));
                } else {
                    return Err(err);
                }
            }
        };

        result |= ((byte & 0x7F) as u64) << shift;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
    }

    Ok(result)
}
