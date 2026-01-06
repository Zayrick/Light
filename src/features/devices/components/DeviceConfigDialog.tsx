import { CloseButton, Dialog, Portal } from "@chakra-ui/react";

export interface DeviceConfigDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function DeviceConfigDialog({ open, onOpenChange }: DeviceConfigDialogProps) {
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
              <Dialog.Title>设备配置</Dialog.Title>
              <Dialog.CloseTrigger asChild>
                <CloseButton size="sm" />
              </Dialog.CloseTrigger>
            </Dialog.Header>
            <Dialog.Body />
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  );
}
