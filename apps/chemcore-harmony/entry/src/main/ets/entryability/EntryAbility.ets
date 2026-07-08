import { AbilityConstant, UIAbility, Want } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { window } from '@kit.ArkUI';

const DOMAIN = 0x0000;

export default class EntryAbility extends UIAbility {
  onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): void {
    hilog.info(DOMAIN, 'ChemCore', 'EntryAbility onCreate');
  }

  onDestroy(): void {
    hilog.info(DOMAIN, 'ChemCore', 'EntryAbility onDestroy');
  }

  onWindowStageCreate(windowStage: window.WindowStage): void {
    windowStage.loadContent('pages/Index', (err) => {
      if (err.code) {
        hilog.error(DOMAIN, 'ChemCore', 'Failed to load content: %{public}s', JSON.stringify(err));
        return;
      }
      try {
        const mainWindow = windowStage.getMainWindowSync();
        mainWindow.setWindowBackgroundColor('#F7F8FA');
        mainWindow.setWindowDecorVisible(false);
        mainWindow.setWindowTitleMoveEnabled(true);
      } catch (error) {
        hilog.warn(DOMAIN, 'ChemCore', 'Failed to configure window chrome: %{public}s', JSON.stringify(error));
      }
      hilog.info(DOMAIN, 'ChemCore', 'Content loaded');
    });
  }

  onWindowStageDestroy(): void {
    hilog.info(DOMAIN, 'ChemCore', 'Window stage destroyed');
  }

  onForeground(): void {
    hilog.info(DOMAIN, 'ChemCore', 'EntryAbility onForeground');
  }

  onBackground(): void {
    hilog.info(DOMAIN, 'ChemCore', 'EntryAbility onBackground');
  }

}
