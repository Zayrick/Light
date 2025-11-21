import { Device } from "../../../types";

interface DeviceDetailProps {
  device: Device;
}

export function DeviceDetail({ device }: DeviceDetailProps) {
  return (
    <>
      <header className="page-header">
        <div>
          <h1 className="page-title">{device.model}</h1>
        </div>
      </header>
      {/* Add more details here */}
    </>
  );
}

