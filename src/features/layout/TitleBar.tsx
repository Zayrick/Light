import { getCurrentWindow } from '@tauri-apps/api/window';
import { usePlatform } from '../../hooks/usePlatform';
import { useMinimizeToTray } from '../../hooks/useMinimizeToTray';
import './TitleBar.css';

export function TitleBar() {
  const appWindow = getCurrentWindow();
  const { isMacOS } = usePlatform();
  const { minimizeToTray } = useMinimizeToTray();

  const minimize = () => {
    if (minimizeToTray) {
      appWindow.hide();
    } else {
      appWindow.minimize();
    }
  };
  const maximize = async () => {
    if (await appWindow.isMaximized()) {
      appWindow.unmaximize();
    } else {
      appWindow.maximize();
    }
  };
  const close = () => appWindow.close();

  return (
    <div className={`titlebar ${isMacOS ? 'titlebar-macos' : ''}`}>
      <div className="titlebar-drag-region" data-tauri-drag-region>
        {!isMacOS && 'Light Studio'}
      </div>
      {!isMacOS && (
        <>
          <div className="titlebar-button" onClick={minimize}>
            <svg className="titlebar-icon" viewBox="0 0 10 1">
              <path d="M0 0h10v1H0z" fill="currentColor" />
            </svg>
          </div>
          <div className="titlebar-button" onClick={maximize}>
            <svg className="titlebar-icon" viewBox="0 0 10 10">
              <path d="M0 0h10v10H0V0zm1 1v8h8V1H1z" fill="currentColor" />
            </svg>
          </div>
          <div className="titlebar-button close" onClick={close}>
            <svg className="titlebar-icon" viewBox="0 0 10 10">
              <path d="M.5 0L0 .5 4.5 5 0 9.5l.5.5L5 5.5 9.5 10l.5-.5L5.5 5 10 .5 9.5 0 5 4.5.5 0z" fill="currentColor" />
            </svg>
          </div>
        </>
      )}
    </div>
  );
}

