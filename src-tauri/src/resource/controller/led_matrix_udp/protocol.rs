//! LED矩阵UDP协议命令定义（与虚拟设备保持一致）

/// 查询设备信息
pub const CMD_QUERY_INFO: u8 = 0x10;
/// 查询设备输出口/布局配置（JSON，可能分片）
pub const CMD_QUERY_CONFIG: u8 = 0x14;
/// 分片帧数据（唯一支持的写入命令）
pub const CMD_FRAGMENT_PIXELS: u8 = 0x12;

/// 当前协议版本
pub const PROTOCOL_VERSION: u8 = 4;
/// 推荐的最大UDP负载（字节），与虚拟设备保持一致
pub const MAX_UDP_PAYLOAD: usize = 1400;

use crate::interface::controller::Color;

/// 设备信息查询结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryInfo {
    pub version: u8,
    pub width: u16,
    pub height: u16,
    pub pixel_size: u16,
    pub name: String,
    pub description: String,
    pub serial: String,
}

/// 配置查询分片
///
/// 响应格式:
/// [cmd, msg_id, total_fragments, fragment_index, data_len_lo, data_len_hi, data_bytes...]
pub struct ConfigFragment<'a> {
    pub msg_id: u8,
    pub total_fragments: u8,
    pub fragment_index: u8,
    pub data: &'a [u8],
}

/// LED矩阵UDP协议编码器/解码器
pub struct LedMatrixProtocol;

impl LedMatrixProtocol {
    /// 编码查询设备信息命令
    #[inline]
    pub fn encode_query_info() -> [u8; 1] {
        [CMD_QUERY_INFO]
    }

    /// 编码查询设备配置命令
    #[inline]
    pub fn encode_query_config() -> [u8; 1] {
        [CMD_QUERY_CONFIG]
    }

    /// 解析设备信息响应
    /// 格式 (strict, v4):
    /// [cmd, version, width_lo, width_hi, height_lo, height_hi, pixel_size_lo, pixel_size_hi,
    ///  name_len, name_bytes,
    ///  desc_len, desc_bytes,
    ///  sn_len, sn_bytes]
    pub fn decode_query_response(data: &[u8]) -> Option<QueryInfo> {
        if data.len() < 9 || data[0] != CMD_QUERY_INFO {
            return None;
        }

        let version = data[1];
        let width = u16::from_le_bytes([data[2], data[3]]);
        let height = u16::from_le_bytes([data[4], data[5]]);
        let pixel_size = u16::from_le_bytes([data[6], data[7]]);
        let mut offset = 8;

        let name_len = *data.get(offset)? as usize;
        offset += 1;
        let name_bytes = data.get(offset..offset + name_len)?;
        offset += name_len;

        let desc_len = *data.get(offset)? as usize;
        offset += 1;
        let desc_bytes = data.get(offset..offset + desc_len)?;
        offset += desc_len;

        let sn_len = *data.get(offset)? as usize;
        offset += 1;
        let sn_bytes = data.get(offset..offset + sn_len)?;

        let name = String::from_utf8_lossy(name_bytes).to_string();
        let description = String::from_utf8_lossy(desc_bytes).to_string();
        let serial = String::from_utf8_lossy(sn_bytes).to_string();

        Some(QueryInfo {
            version,
            width,
            height,
            pixel_size,
            name,
            description,
            serial,
        })
    }

    /// 解析配置分片响应
    pub fn decode_config_fragment(data: &[u8]) -> Option<ConfigFragment<'_>> {
        if data.len() < 6 || data[0] != CMD_QUERY_CONFIG {
            return None;
        }

        let msg_id = data[1];
        let total_fragments = data[2];
        let fragment_index = data[3];
        let data_len = u16::from_le_bytes([data[4], data[5]]) as usize;

        if data.len() < 6 + data_len {
            return None;
        }

        Some(ConfigFragment {
            msg_id,
            total_fragments,
            fragment_index,
            data: &data[6..6 + data_len],
        })
    }

    /// 计算单个分片最多可携带的像素数量
    /// header = cmd(1) + frame_id(1) + total_fragments(1) + fragment_index(1) + count(2) = 6
    #[inline]
    pub fn max_pixels_per_fragment(max_payload: usize) -> Result<usize, String> {
        if max_payload <= 6 {
            return Err("Max UDP payload is too small for fragment header".to_string());
        }
        Ok((max_payload - 6) / 5)
    }

    /// 计算总分片数，限制在协议约定的 u8 范围内
    #[inline]
    pub fn calc_total_fragments(color_count: usize, max_pixels_per_fragment: usize) -> Result<u8, String> {
        if max_pixels_per_fragment == 0 {
            return Err("max_pixels_per_fragment cannot be zero".to_string());
        }
        let total = color_count.div_ceil(max_pixels_per_fragment);
        u8::try_from(total).map_err(|_| "Fragment count exceeds protocol limit (<=255)".to_string())
    }

    /// 编码单个分片命令到缓冲区
    /// 格式: [cmd, frame_id, total_fragments, fragment_index, count_lo, count_hi, (index_lo, index_hi, r, g, b) * count]
    pub fn encode_fragment_into(
        frame_id: u8,
        total_fragments: u8,
        fragment_index: u8,
        start_index: usize,
        colors: &[Color],
        buffer: &mut Vec<u8>,
    ) -> Result<(), String> {
        if colors.is_empty() {
            return Ok(());
        }

        if colors.len() > u16::MAX as usize {
            return Err("Fragment pixel count exceeds protocol limit".to_string());
        }

        buffer.clear();
        buffer.reserve(1 + 5 + colors.len() * 5);

        buffer.push(CMD_FRAGMENT_PIXELS);
        buffer.push(frame_id);
        buffer.push(total_fragments);
        buffer.push(fragment_index);
        buffer.extend_from_slice(&(colors.len() as u16).to_le_bytes());

        let mut index = start_index;
        for color in colors {
            let idx: u16 = index
                .try_into()
                .map_err(|_| "LED index exceeds u16 range for protocol".to_string())?;
            buffer.extend_from_slice(&idx.to_le_bytes());
            buffer.push(color.r);
            buffer.push(color.g);
            buffer.push(color.b);
            index += 1;
        }

        Ok(())
    }
}
