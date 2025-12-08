/// LED矩阵UDP协议命令定义（与虚拟设备保持一致）

/// 查询设备信息
pub const CMD_QUERY_INFO: u8 = 0x10;
/// 批量更新像素（线性索引 + RGB）
pub const CMD_UPDATE_PIXELS: u8 = 0x11;

/// 当前协议版本
pub const PROTOCOL_VERSION: u8 = 3;

use crate::interface::controller::Color;

/// 设备信息查询结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryInfo {
    pub version: u8,
    pub width: u16,
    pub height: u16,
    pub pixel_size: u16,
    pub name: String,
}

/// LED矩阵UDP协议编码器/解码器
pub struct LedMatrixProtocol;

impl LedMatrixProtocol {
    /// 编码查询设备信息命令
    #[inline]
    pub fn encode_query_info() -> [u8; 1] {
        [CMD_QUERY_INFO]
    }

    /// 解析设备信息响应
    /// 格式: [cmd, version, width_lo, width_hi, height_lo, height_hi, pixel_size_lo, pixel_size_hi, name_len, name_bytes]
    pub fn decode_query_response(data: &[u8]) -> Option<QueryInfo> {
        if data.len() < 9 || data[0] != CMD_QUERY_INFO {
            return None;
        }

        let version = data[1];
        let width = u16::from_le_bytes([data[2], data[3]]);
        let height = u16::from_le_bytes([data[4], data[5]]);
        let pixel_size = u16::from_le_bytes([data[6], data[7]]);
        let name_len = data[8] as usize;

        if data.len() < 9 + name_len {
            return None;
        }

        let name_bytes = &data[9..9 + name_len];
        let name = String::from_utf8_lossy(name_bytes).to_string();

        Some(QueryInfo {
            version,
            width,
            height,
            pixel_size,
            name,
        })
    }

    /// 编码批量更新像素命令（写入已有缓冲区以减少分配）
    /// 格式: [cmd, count_lo, count_hi, (index_lo, index_hi, r, g, b) * count]
    pub fn encode_update_pixels_into(
        colors: &[Color],
        buffer: &mut Vec<u8>,
    ) -> Result<(), String> {
        let count = colors.len();

        if count > u16::MAX as usize {
            return Err(format!(
                "Color count {} exceeds protocol limit {}",
                count,
                u16::MAX
            ));
        }

        buffer.clear();
        buffer.reserve(1 + 2 + count * 5);

        buffer.push(CMD_UPDATE_PIXELS);
        buffer.extend_from_slice(&(count as u16).to_le_bytes());

        for (idx, color) in colors.iter().enumerate() {
            let index: u16 = idx
                .try_into()
                .map_err(|_| "LED index exceeds u16 range for protocol".to_string())?;

            buffer.extend_from_slice(&index.to_le_bytes());
            buffer.push(color.r);
            buffer.push(color.g);
            buffer.push(color.b);
        }

        Ok(())
    }
}
