Rust相关Windows API可以通过Context7工具搜索"Rust for Windows"得到

屏幕选择策略：
- 后端仅提供显示器元数据（index/name/分辨率等），不再负责 UI 逻辑。
- 前端使用下拉菜单渲染并推送 `displayIndex` 参数。
- 如需扩展其它动态参数，优先让后端返回参数描述，再由前端渲染控件。