; Per-user installer (no admin prompt). Built by package.ps1; needs Inno Setup 6
; (winget install JRSoftware.InnoSetup). Packs whatever package.ps1 staged in dist\visualiser.
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{71CCD7C8-D92F-4643-A58F-EFFADAF0AF2D}
AppName=Visualiser
AppVersion={#AppVersion}
AppPublisher=Prathamesh Kulkarni
DefaultDirName={localappdata}\Programs\Visualiser
DisableProgramGroupPage=yes
DefaultGroupName=Visualiser
PrivilegesRequired=lowest
ChangesEnvironment=yes
OutputDir=..\dist
OutputBaseFilename=visualiser-setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\visualiser.exe

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut (you can also drop audio files on it)"
Name: contextmenu; Description: "Add ""Make visuals"" to the right-click menu of audio files"

[Files]
Source: "..\dist\visualiser\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{group}\Visualiser"; Filename: "{app}\visualiser.exe"; WorkingDir: "{app}"
Name: "{group}\Uninstall Visualiser"; Filename: "{uninstallexe}"
Name: "{userdesktop}\Visualiser"; Filename: "{app}\visualiser.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wav\shell\Visualiser"; ValueType: string; ValueName: ""; ValueData: "Make visuals"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wav\shell\Visualiser\command"; ValueType: string; ValueName: ""; ValueData: """{app}\visualiser.exe"" ""%1"""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flac\shell\Visualiser"; ValueType: string; ValueName: ""; ValueData: "Make visuals"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flac\shell\Visualiser\command"; ValueType: string; ValueName: ""; ValueData: """{app}\visualiser.exe"" ""%1"""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp3\shell\Visualiser"; ValueType: string; ValueName: ""; ValueData: "Make visuals"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp3\shell\Visualiser\command"; ValueType: string; ValueName: ""; ValueData: """{app}\visualiser.exe"" ""%1"""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.aiff\shell\Visualiser"; ValueType: string; ValueName: ""; ValueData: "Make visuals"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.aiff\shell\Visualiser\command"; ValueType: string; ValueName: ""; ValueData: """{app}\visualiser.exe"" ""%1"""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.aif\shell\Visualiser"; ValueType: string; ValueName: ""; ValueData: "Make visuals"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.aif\shell\Visualiser\command"; ValueType: string; ValueName: ""; ValueData: """{app}\visualiser.exe"" ""%1"""; Tasks: contextmenu

[Code]
const EnvKey = 'Environment';

// Position of Dir in the ';'-separated Paths (1-based index into Paths), 0 if absent.
function PathPos(Paths, Dir: string): Integer;
begin
  Result := Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(Paths) + ';');
end;

procedure CurStepChanged(CurStep: TSetupStep);
var Paths: string;
begin
  if CurStep = ssPostInstall then begin
    if not RegQueryStringValue(HKCU, EnvKey, 'Path', Paths) then Paths := '';
    if PathPos(Paths, ExpandConstant('{app}')) = 0 then begin
      if (Paths <> '') and (Copy(Paths, Length(Paths), 1) <> ';') then Paths := Paths + ';';
      RegWriteExpandStringValue(HKCU, EnvKey, 'Path', Paths + ExpandConstant('{app}'));
    end;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var Paths, Dir: string; P: Integer;
begin
  if CurUninstallStep = usPostUninstall then begin
    if RegQueryStringValue(HKCU, EnvKey, 'Path', Paths) then begin
      Dir := ExpandConstant('{app}');
      P := PathPos(Paths, Dir);
      if P > 0 then begin
        Delete(Paths, P, Length(Dir));
        if P > 1 then Delete(Paths, P - 1, 1)
        else if Copy(Paths, 1, 1) = ';' then Delete(Paths, 1, 1);
        RegWriteExpandStringValue(HKCU, EnvKey, 'Path', Paths);
      end;
    end;
  end;
end;
