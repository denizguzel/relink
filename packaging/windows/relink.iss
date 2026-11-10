; Re-Link per-user installer (Inno Setup 6). Built by the release workflow:
;   ISCC /DAppVersion=0.3.0 /DExe=relink.exe /DOutputDir=dist packaging\windows\relink.iss
; Never needs administrator rights: it installs for the current user only,
; under %LOCALAPPDATA%\Programs, and registers itself under HKCU.
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef Exe
  #define Exe "relink.exe"
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif

[Setup]
AppId={{6F1D2B5A-3C47-4E0B-9A58-2D7E41C0B9A3}
AppName=Re-Link
; Apps & features shows this name; without it Inno appends "version <AppVersion>".
AppVerName=Re-Link
AppVersion={#AppVersion}
AppPublisher=Re-Link
AppPublisherURL=https://github.com/denizguzel/relink
AppSupportURL=https://github.com/denizguzel/relink/issues
DefaultDirName={userpf}\Re-Link
DefaultGroupName=Re-Link
DisableProgramGroupPage=yes
; "lowest" and no override (the default): Setup never asks for elevation, even if started
; as administrator it keeps the per-user install.
PrivilegesRequired=lowest
UsePreviousPrivileges=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
Compression=lzma2
SolidCompression=yes
UninstallDisplayIcon={app}\relink.exe
OutputDir={#OutputDir}
OutputBaseFilename=relink-{#AppVersion}-windows-x64-setup
WizardStyle=modern
SetupIconFile=..\..\assets\brand\relink.ico
WizardImageFile=..\..\assets\brand\installer-wizard.bmp
WizardSmallImageFile=..\..\assets\brand\installer-small.bmp

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl"

[Files]
Source: "{#Exe}"; DestDir: "{app}"; DestName: "relink.exe"; Flags: ignoreversion

[Tasks]
; Ticked by default, so a silent install makes the shortcut too.
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Icons]
Name: "{userprograms}\Re-Link"; Filename: "{app}\relink.exe"
Name: "{userdesktop}\Re-Link"; Filename: "{app}\relink.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\relink.exe"; Description: "{cm:LaunchProgram,Re-Link}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Left behind by the in-app updater.
Type: files; Name: "{app}\relink.exe.old"
Type: files; Name: "{app}\relink.exe.new"

[Code]
// Re-Link lives in the tray, so a running copy would keep its file locked.
procedure StopRelink();
var
  Code: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/IM relink.exe /F', '', SW_HIDE, ewWaitUntilTerminated, Code);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssInstall then StopRelink();
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then StopRelink();
end;
