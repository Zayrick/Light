import { CloseButton, Dialog, HStack, Portal, Separator, Spinner, Stack, Text } from "@chakra-ui/react";
import { useEffect, useState } from "react";
import type { Device, OutputPort } from "../../../types";
import { api } from "../../../services/api";

export interface DeviceConfigDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** 设备端口标识，用于获取设备详情 */
  port?: string;
  deviceName?: string;
}

export function DeviceConfigDialog({ open, onOpenChange, port, deviceName }: DeviceConfigDialogProps) {
  const [device, setDevice] = useState<Device | null>(null);
  const [loading, setLoading] = useState(false);

  // 弹窗打开时，根据 port 获取设备信息
  useEffect(() => {
    if (open && port) {
      setLoading(true);
      api.getDevice(port)
        .then(setDevice)
        .catch(() => setDevice(null))
        .finally(() => setLoading(false));
    } else if (!open) {
      // 弹窗关闭时清空状态
      setDevice(null);
    }
  }, [open, port]);

  // 筛选可编辑 segment 的 output
  const editableOutputs: OutputPort[] = device?.outputs.filter((o) => o.capabilities.editable) ?? [];

  // 标题：有设备名时显示 "设备名 · 配置"，否则只显示 "配置"
  const title = deviceName ? `${deviceName} · 配置` : "配置";

  return (
    <Dialog.Root
      lazyMount
      unmountOnExit
      placement="center"
      open={open}
      onOpenChange={(e) => onOpenChange(e.open)}
    >
      <Portal>
        <Dialog.Backdrop />
        <Dialog.Positioner>
          <Dialog.Content>
            <Dialog.Header>
              <Dialog.Title>{title}</Dialog.Title>
            </Dialog.Header>
            <Dialog.Body>
              <HStack mb="3">
                <Text flexShrink="0" fontWeight="medium">Segment 编辑</Text>
                <Separator flex="1" />
              </HStack>
              {loading ? (
                <Spinner size="sm" />
              ) : editableOutputs.length > 0 ? (
                <Stack gap="1">
                  {editableOutputs.map((output) => (
                    <Text key={output.id} fontSize="sm" color="var(--text-secondary)">
                      {output.name}
                    </Text>
                  ))}
                </Stack>
              ) : (
                <Text fontSize="sm" color="var(--text-tertiary)">
                  没有可编辑的输出口
                </Text>
              )}
            </Dialog.Body>
            <Dialog.CloseTrigger asChild>
              <CloseButton size="sm" />
            </Dialog.CloseTrigger>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  );
}
