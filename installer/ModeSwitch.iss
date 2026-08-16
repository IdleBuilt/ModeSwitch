; ModeSwitch installer
; Build with: iscc installer\ModeSwitch.iss
; Requires target\release\ModeSwitch.exe (cargo build --release)

#define AppName    "ModeSwitch"
#define AppVersion "0.1"
#define AppExe     "ModeSwitch.exe"
#define Publisher  "KiraiEEE"
#define AppUrl     "https://github.com/IdleBuilt/ModeSwitch"

[Setup]
AppId={{6C3703F3-6E06-490C-AB42-F7224D24A801}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
VersionInfoVersion=0.1.0.0
AppPublisher={#Publisher}
AppPublisherURL={#AppUrl}
AppSupportURL={#AppUrl}/issues
AppUpdatesURL={#AppUrl}/releases

; C:\Program Files\KiraiEEE\ModeSwitch
DefaultDirName={commonpf}\{#Publisher}\{#AppName}
DefaultGroupName={#AppName}
UninstallDisplayName={#AppName}
UninstallDisplayIcon={app}\{#AppExe}

PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

; A quiet, quick wizard: tasks -> install -> done
WizardStyle=modern
DisableWelcomePage=yes
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=yes
ShowLanguageDialog=no
SetupIconFile=..\res\app.ico

; Maximum compression
Compression=lzma2/ultra64
InternalCompressLevel=ultra64
SolidCompression=yes
LZMAUseSeparateProcess=yes
LZMANumFastBytes=273
CompressionThreads=auto

OutputDir=Output
OutputBaseFilename={#AppName}-Setup-v{#AppVersion}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "startupicon";   Description: "Run {#AppName} when Windows starts"; GroupDescription: "Options:"
Name: "startmenuicon"; Description: "Create a Start menu shortcut";       GroupDescription: "Options:"
Name: "desktopicon";   Description: "Create a desktop shortcut";          GroupDescription: "Options:"; Flags: unchecked

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}";           Filename: "{app}\{#AppExe}"; Tasks: startmenuicon
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}";  Tasks: startmenuicon
Name: "{commondesktop}\{#AppName}";   Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; Matches the value name/format the tray's "Start with Windows" menu item uses,
; so the two stay in sync. Quoted because the path contains spaces.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; \
    ValueType: string; ValueName: "{#AppName}"; ValueData: """{app}\{#AppExe}"""; \
    Flags: uninsdeletevalue; Tasks: startupicon

[Run]
Filename: "{app}\{#AppExe}"; Description: "Launch {#AppName}"; \
    Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#AppExe}"; \
    Flags: runhidden; RunOnceId: "StopModeSwitch"

[Code]
{ Close a running instance before overwriting it, instead of making the user do it. }
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Result := '';
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#AppExe}', '',
       SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

{ The tray menu can add the Run entry after install, so always clear it on uninstall
  regardless of whether the startup task was ticked. }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RegDeleteValue(HKEY_CURRENT_USER,
      'Software\Microsoft\Windows\CurrentVersion\Run', '{#AppName}');
end;
