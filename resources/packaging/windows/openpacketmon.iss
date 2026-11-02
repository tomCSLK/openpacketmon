; Installer of openpacketmon for Windows (Inno Setup 6, https://jrsoftware.org/isinfo.php).
; Built by dev/build-installer.ps1, which builds the programs first and passes the version from Cargo.toml.
; Npcap is never part of it (its license does not allow that): the installer only says where to get it.

#ifndef AppVersion
  #error AppVersion is missing; build the installer with dev/build-installer.ps1
#endif

#define Root "..\..\.."

[Setup]
; identifies the program for updates and uninstalling; never change it
AppId={{758FE70C-DE1E-4936-8905-8CAEF9110ED6}
AppName=openpacketmon
AppVersion={#AppVersion}
AppVerName=openpacketmon {#AppVersion}
AppPublisher=Thomas Cieslik
VersionInfoVersion={#AppVersion}
VersionInfoDescription=openpacketmon Setup
; for the current user without administrator rights by default, for all users on request
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
DefaultDirName={autopf}\openpacketmon
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
; closes a running openpacketmon before an update replaces it
CloseApplications=yes
OutputDir={#Root}\target\installer
OutputBaseFilename=openpacketmon-{#AppVersion}-setup
SetupIconFile=openpacketmon.ico
UninstallDisplayIcon={app}\openpacketmon.exe
UninstallDisplayName=openpacketmon
WizardStyle=modern
Compression=lzma2
SolidCompression=yes

[Languages]
Name: "de"; MessagesFile: "compiler:Languages\German.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#Root}\target\release\openpacketmon.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\target\release\openpacketmon-capture.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\LICENSE-MIT"; DestDir: "{app}"; DestName: "LICENSE-MIT.txt"; Flags: ignoreversion
Source: "{#Root}\LICENSE-APACHE"; DestDir: "{app}"; DestName: "LICENSE-APACHE.txt"; Flags: ignoreversion
Source: "{#Root}\NOTICE"; DestDir: "{app}"; DestName: "NOTICE.txt"; Flags: ignoreversion
Source: "{#Root}\THIRD-PARTY-LICENSES.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\openpacketmon"; Filename: "{app}\openpacketmon.exe"
Name: "{autodesktop}\openpacketmon"; Filename: "{app}\openpacketmon.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\openpacketmon.exe"; Description: "{cm:LaunchProgram,openpacketmon}"; Flags: nowait postinstall skipifsilent; Check: NpcapInstalled

; The settings in %APPDATA%\openpacketmon stay when uninstalling, as for an update.

[CustomMessages]
de.NpcapTitle=Npcap fehlt noch
de.NpcapSubtitle=openpacketmon braucht Npcap, um Netzwerkverkehr mitzuschneiden.
de.NpcapText=Npcap gehört nicht zu openpacketmon und muss selbst installiert werden. Die kostenlose Ausgabe darf auf bis zu fünf Rechnern für den eigenen Gebrauch eingesetzt werden, die Bedingungen stehen auf npcap.com.%n%nBeim Installieren von Npcap „Install Npcap in WinPcap API-compatible Mode“ angehakt lassen. Danach openpacketmon starten.%n%nopenpacketmon wird trotzdem installiert.
de.NpcapButton=Npcap-Downloadseite öffnen
en.NpcapTitle=Npcap is still missing
en.NpcapSubtitle=openpacketmon needs Npcap to capture network traffic.
en.NpcapText=Npcap is not part of openpacketmon and has to be installed separately. Its free edition may be used on up to five computers for your own use; the terms are on npcap.com.%n%nWhen installing Npcap, leave “Install Npcap in WinPcap API-compatible Mode” checked. Then start openpacketmon.%n%nopenpacketmon is installed anyway.
en.NpcapButton=Open the Npcap download page

[Code]
// where Npcap puts wpcap.dll in its WinPcap API-compatible Mode, where openpacketmon looks for it
function NpcapInstalled: Boolean;
begin
  Result := FileExists(ExpandConstant('{sys}\wpcap.dll'));
end;

procedure OpenNpcapPage(Sender: TObject);
var
  Code: Integer;
begin
  ShellExec('open', 'https://npcap.com/#download', '', '', SW_SHOWNORMAL, ewNoWait, Code);
end;

procedure InitializeWizard;
var
  Page: TOutputMsgWizardPage;
  Button: TNewButton;
begin
  if NpcapInstalled then
    Exit;
  // after the installation, before the last page
  Page := CreateOutputMsgPage(wpInfoAfter, CustomMessage('NpcapTitle'), CustomMessage('NpcapSubtitle'),
    CustomMessage('NpcapText'));
  Button := TNewButton.Create(Page);
  Button.Parent := Page.Surface;
  Button.Caption := CustomMessage('NpcapButton');
  Button.Width := ScaleX(240);
  Button.Height := ScaleY(28);
  Button.Top := Page.Surface.Height - Button.Height;
  Button.OnClick := @OpenNpcapPage;
end;
