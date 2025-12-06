/// LED矩阵UDP协议命令定义
/// 基于Python客户端的协议实现

/// 刷新显示
pub const CMD_REFRESH: u8 = 0xFF;
/// 填充整个屏幕
pub const CMD_FILL_SCREEN: u8 = 0x03;
/// 全量帧更新
pub const CMD_FULL_FRAME: u8 = 0x06;
/// 全量帧更新并刷新
pub const CMD_FULL_FRAME_REFRESH: u8 = 0xFE;

use crate::interface::controller::Color;

/// LED矩阵UDP协议编码器
pub struct LedMatrixProtocol;

impl LedMatrixProtocol {
    /// 编码填充屏幕命令
    pub fn encode_fill_screen(color: Color) -> Vec<u8> {
        vec![CMD_FILL_SCREEN, color.r, color.g, color.b]
    }

    /// 编码刷新命令
    pub fn encode_refresh() -> Vec<u8> {
        vec![CMD_REFRESH]
    }

    /// 编码全量帧更新命令（到现有缓冲区）
    /// 避免分配新内存，提高性能
    pub fn encode_full_frame_into(colors: &[Color], auto_refresh: bool, buffer: &mut Vec<u8>) {
        let cmd = if auto_refresh {
            CMD_FULL_FRAME_REFRESH
        } else {
            CMD_FULL_FRAME
        };
        buffer.clear();
        buffer.reserve(1 + colors.len() * 3);
        buffer.push(cmd);
        for color in colors {
            buffer.push(color.r);
            buffer.push(color.g);
            buffer.push(color.b);
        }
    }
}
