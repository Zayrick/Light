import { Box, CloseButton, Dialog, HStack, Portal, Text } from "@chakra-ui/react";

export interface DeviceConfigDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  deviceName?: string;
}

export function DeviceConfigDialog({ open, onOpenChange, deviceName }: DeviceConfigDialogProps) {
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
              <HStack justify="space-between" align="start" gap="4" w="full">
                <Box>
                  <Dialog.Title>配置</Dialog.Title>
                  {deviceName ? (
                    <Text mt="1" fontSize="sm" color="var(--text-secondary)" lineHeight="1.3">
                      {deviceName}
                    </Text>
                  ) : null}
                </Box>

                <Dialog.CloseTrigger asChild>
                  <CloseButton size="sm" />
                </Dialog.CloseTrigger>
              </HStack>
            </Dialog.Header>
            <Dialog.Body />
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  );
}
