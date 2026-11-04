#ifndef AppVersion
  #error AppVersion is required
#endif
[Setup]
AppId={{A47BC568-3729-4F18-8420-949A125CB746}
AppName=龙泡泡 · My Dragon Bubble
AppVersion={#AppVersion}
AppPublisher=Dragon Bubble
DefaultDirName={localappdata}\Programs\Dragon Bubble
DefaultGroupName=龙泡泡
PrivilegesRequired=lowest
DisableDirPage=no
DisableProgramGroupPage=yes
WizardStyle=modern dynamic windows11
WizardSizePercent=120
WizardBackColor=$F3F3F3
WizardBackColorDynamicDark=$202020
WizardSmallImageBackColor=$F3F3F3
WizardSmallImageBackColorDynamicDark=$202020
WizardImageBackColor=$F3F3F3
WizardImageBackColorDynamicDark=$202020
SetupIconFile=..\assets\tray.ico
UninstallDisplayIcon={app}\My Dragon Bubble.exe
OutputDir=..\outputs\installers
OutputBaseFilename=dragon-bubble-v{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
RestartApplications=no
AppMutex=DragonBubbleDesktopPet
[LangOptions]
DialogFontName=Microsoft YaHei UI
[Languages]
Name: "chinesesimp"; MessagesFile: "ChineseSimplified.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"
[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; Flags: unchecked
[Files]
Source: "..\outputs\rust\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
[Icons]
Name: "{group}\龙泡泡"; Filename: "{app}\My Dragon Bubble.exe"
Name: "{autodesktop}\龙泡泡"; Filename: "{app}\My Dragon Bubble.exe"; Tasks: desktopicon
[Run]
Filename: "{app}\My Dragon Bubble.exe"; Description: "启动龙泡泡"; Flags: nowait postinstall skipifsilent


Filename: "{app}\My Dragon Bubble.exe"; Flags: nowait; Check: IsUpdate
[Code]
function IsUpdate: Boolean;
begin
  Result := Pos('/UPDATE', Uppercase(GetCmdTail)) > 0;
end;
