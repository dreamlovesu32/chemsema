import { AbilityConstant, UIAbility, Want } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { window } from '@kit.ArkUI';

const DOMAIN = 0x0000;

export default class EntryAbility extends UIAbility {
  onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): void {
    hilog.info(DOMAIN, 'ChemSema', 'EntryAbility onCreate');
  }

  onDestroy(): void {
    hilog.info(DOMAIN, 'ChemSema', 'EntryAbility onDestroy');
  }

  onWindowStageCreate(windowStage: window.WindowStage): void {
    windowStage.loadContent('pages/Index', (err) => {
      if (err.code) {
        hilog.error(DOMAIN, 'ChemSema', 'Failed to load content: %{public}s', JSON.stringify(err));
        return;
      }
      try {
        const mainWindow = windowStage.getMainWindowSync();
        mainWindow.setWindowBackgroundColor('#F7F8FA');
        mainWindow.setWindowDecorVisible(false);
        mainWindow.setWindowTitleMoveEnabled(true);
      } catch (error) {
        hilog.warn(DOMAIN, 'ChemSema', 'Failed to configure window chrome: %{public}s', JSON.stringify(error));
      }
      hilog.info(DOMAIN, 'ChemSema', 'Content loaded');
    });
  }

  onWindowStageDestroy(): void {
    hilog.info(DOMAIN, 'ChemSema', 'Window stage destroyed');
  }

  onForeground(): void {
    hilog.info(DOMAIN, 'ChemSema', 'EntryAbility onForeground');
  }

  onBackground(): void {
    hilog.info(DOMAIN, 'ChemSema', 'EntryAbility onBackground');
  }

}
