"""Protocol constants for the TestDevice virtual device.

This test device is intentionally *not* backward-compatible.
Only the current protocol is supported.
"""

CMD_QUERY_INFO = 0x10
CMD_FRAGMENT_PIXELS = 0x12
CMD_QUERY_CONFIG = 0x14

PROTOCOL_VERSION = 4

MAX_UDP_PAYLOAD = 1400
