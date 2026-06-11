➡️ [UI Test Specifications](../../Tests/UITest/README.md) · [Environment](../../Tests/UITest/environment/hyper-v-vm-ee-test.md)

# Skill — hyperv-operation

Use this skill whenever an agent needs to operate Hyper-V for EasyEnglish UI
testing. It covers host readiness, VM creation/import, lifecycle operations,
guest login/control, software deployment, UI automation constraints, checkpoint
handling, troubleshooting, and required reporting.

## 1. Safety rules

1. Do not enable Hyper-V, reboot the host, create or modify VMs, or download
   large ISO/VM files silently. Make the action visible to the user first.
2. Do not bypass Windows licensing or activation requirements.
3. Prefer official Microsoft sources for ISO or VM packages.
4. Reuse suitable files from the user's Downloads directory before downloading
   anything new.
5. Keep VM operations scoped to the dedicated VM named `vm-ee-test`.
6. Do not use the host desktop as the default UI test target.
7. Do not run UI automation as a Windows service. Services run in Session 0 and
   cannot reliably interact with the user's desktop.

## 2. Host readiness checks

Run these checks before creating, repairing, or using `vm-ee-test`.

```powershell
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) { throw "Hyper-V operations require an Administrator shell." }

Get-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V-All |
    Select-Object FeatureName, State

Get-Command Get-VM, New-VM, Get-VMSwitch -ErrorAction SilentlyContinue |
    Select-Object Name, Source

Get-VM -Name vm-ee-test -ErrorAction SilentlyContinue |
    Select-Object Name, State, Generation, Uptime, Status

Get-VMSwitch |
    Select-Object Name, SwitchType, NetAdapterInterfaceDescription

$downloads = Join-Path $env:USERPROFILE "Downloads"
Get-ChildItem $downloads -File |
    Where-Object { $_.Extension -in ".iso", ".ISO", ".zip" -or $_.Name -match "Windows|Win11|WinDev|HyperV" } |
    Select-Object Name, Length, LastWriteTime, FullName

Get-PSDrive -Name (Get-Item $downloads).PSDrive.Name |
    Select-Object Name, Used, Free
```

Expected state for normal UI testing:

- Hyper-V feature is `Enabled`.
- Hyper-V cmdlets are available.
- `vm-ee-test` exists.
- A switch such as `Default Switch` exists.
- There is enough disk space for VM disks, checkpoints, logs, and screenshots.

## 3. VM acquisition strategy

Use this order:

1. Reuse a suitable existing ISO or Hyper-V VM package in Downloads.
2. Prefer a deterministic Windows ISO + unattend flow for long-term automated UI
   tests. The unattend path must create a known local admin account for
   PowerShell Direct.
3. Use the official Microsoft Windows development Hyper-V VM package only as a
   fallback or quick bootstrap. It may boot successfully but still lack known
   usable credentials for automation.
4. Do not depend on reverse-engineered Microsoft download connector calls. If
   Microsoft Sentinel rejects an automated ISO link request, stop using that path
   and report the blocker.

Observed local result:

- `aka.ms/windev_VM_hyperv` redirected to an official Microsoft Hyper-V package.
- The downloaded package contained a VHDX that could boot as `vm-ee-test`.
- Heartbeat and KVP were OK.
- PowerShell Direct login with common default credentials failed.
- Offline VHD inspection showed `Users\User`, but the unattend password had
  been removed. This is why deterministic credential provisioning is required.

## 4. VM creation/import operations

If using a VHDX package:

```powershell
$vmName = "vm-ee-test"
$vmRoot = "C:\HyperV\vm-ee-test"
$vhdDir = Join-Path $vmRoot "Virtual Hard Disks"
$vhdPath = Join-Path $vhdDir "vm-ee-test.vhdx"

New-Item -ItemType Directory -Force -Path $vhdDir | Out-Null

$switch = Get-VMSwitch -Name "Default Switch" -ErrorAction Stop

New-VM -Name $vmName `
    -Generation 2 `
    -MemoryStartupBytes 8GB `
    -VHDPath $vhdPath `
    -Path $vmRoot `
    -SwitchName $switch.Name

Set-VMProcessor -VMName $vmName -Count 4
Set-VMMemory -VMName $vmName -DynamicMemoryEnabled $true -MinimumBytes 4GB -StartupBytes 8GB -MaximumBytes 12GB
Set-VMFirmware -VMName $vmName -EnableSecureBoot On -SecureBootTemplate MicrosoftWindows

try {
    Set-VMKeyProtector -VMName $vmName -NewLocalKeyProtector
    Enable-VMTPM -VMName $vmName
} catch {
    Write-Warning "TPM setup failed or is unavailable: $($_.Exception.Message)"
}

Enable-VMIntegrationService -VMName $vmName -Name "Guest Service Interface"
```

If using an ISO, prefer an unattended installation that creates a known local
admin account and enables auto-login for the test desktop session. Do not rely
on manual OOBE for repeatable UI tests.

## 5. VM lifecycle operations

Start and wait for heartbeat:

```powershell
Start-VM -Name vm-ee-test

$deadline = (Get-Date).AddMinutes(5)
do {
    Start-Sleep -Seconds 10
    $vm = Get-VM -Name vm-ee-test
    $heartbeat = Get-VMIntegrationService -VMName vm-ee-test -Name "Heartbeat"
    if ($vm.State -eq "Running" -and $heartbeat.PrimaryStatusDescription -eq "OK") {
        break
    }
} while ((Get-Date) -lt $deadline)
```

Inspect VM state:

```powershell
Get-VM -Name vm-ee-test |
    Select-Object Name, State, Uptime, Status, CPUUsage, MemoryAssigned

Get-VMIntegrationService -VMName vm-ee-test |
    Select-Object Name, Enabled, PrimaryStatusDescription, SecondaryStatusDescription

Get-VMNetworkAdapter -VMName vm-ee-test |
    Select-Object VMName, SwitchName, Status, IPAddresses

Get-VMVideo -VMName vm-ee-test
```

Open visual console for manual inspection:

```powershell
Start-Process "$env:WINDIR\System32\vmconnect.exe" -ArgumentList "$env:COMPUTERNAME vm-ee-test"
```

Capture a machine-readable VM screenshot from the host with Hyper-V WMI. The
method returns RGB565 raw pixels with a four-byte header; decode it before
saving as PNG.

```powershell
$vmName = "vm-ee-test"
$width = 1024
$height = 768
$out = "$env:TEMP\vm-ee-test.png"

$vm = Get-CimInstance -Namespace root\virtualization\v2 -ClassName Msvm_ComputerSystem |
    Where-Object ElementName -eq $vmName
$service = Get-CimInstance -Namespace root\virtualization\v2 -ClassName Msvm_VirtualSystemManagementService
$shot = Invoke-CimMethod -InputObject $service `
    -MethodName GetVirtualSystemThumbnailImage `
    -Arguments @{ TargetSystem = $vm; WidthPixels = $width; HeightPixels = $height }

if ($shot.ReturnValue -ne 0 -or -not $shot.ImageData) {
    throw "Hyper-V thumbnail capture failed: $($shot.ReturnValue)"
}

Add-Type -AssemblyName System.Drawing
$bitmap = [System.Drawing.Bitmap]::new($width, $height, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
$offset = 4
for ($y = 0; $y -lt $height; $y++) {
    for ($x = 0; $x -lt $width; $x++) {
        $pixel = [int]($shot.ImageData[$offset] -bor ($shot.ImageData[$offset + 1] -shl 8))
        $r = [int][math]::Round(((($pixel -shr 11) -band 0x1F) * 255 / 31))
        $g = [int][math]::Round(((($pixel -shr 5) -band 0x3F) * 255 / 63))
        $b = [int][math]::Round((($pixel -band 0x1F) * 255 / 31))
        $bitmap.SetPixel($x, $y, [System.Drawing.Color]::FromArgb($r, $g, $b))
        $offset += 2
    }
}
$bitmap.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$bitmap.Dispose()
```

Inject simple keyboard input from the host when the VM console needs to be
woken. This is useful for validation and recovery, not a replacement for a
proper guest-side UI runner.

```powershell
$vm = Get-WmiObject -Namespace root\virtualization\v2 -Class Msvm_ComputerSystem |
    Where-Object { $_.ElementName -eq "vm-ee-test" }
$keyboard = Get-WmiObject -Namespace root\virtualization\v2 `
    -Query "SELECT * FROM Msvm_Keyboard WHERE SystemName='$($vm.Name)'"

$keyboard.TypeKey([uint16]13)       # Enter
$keyboard.TypeKey([uint16]32)       # Space
$keyboard.TypeCtrlAltDel()          # Security screen
```

Observed validation on the local `vm-ee-test`:

- VM start, heartbeat, KVP, network IP, and video queries worked.
- `Copy-VMFile` from host to `C:\Users\Public\Desktop\ee-hyperv-copy-test.txt`
  worked after enabling Guest Service Interface.
- WMI screenshot capture worked and showed the Windows desktop.
- Keyboard injection worked; `TypeCtrlAltDel()` moved the guest to the Windows
  security screen.
- A later screenshot showed black/security-screen states, so screenshot capture
  alone does not prove the desktop is unlocked or ready for UI automation.

Stop safely when possible:

```powershell
Stop-VM -Name vm-ee-test
```

Force power-off only when the VM is stuck and the user accepts potential guest
state loss:

```powershell
Stop-VM -Name vm-ee-test -TurnOff
```

## 6. Guest login and PowerShell Direct

PowerShell Direct is the preferred host-to-guest control channel. It does not
require guest networking or WinRM setup, but it does require valid guest
credentials and a configured user profile.

Wait for PowerShell Direct:

```powershell
$credential = Get-Credential
$deadline = (Get-Date).AddMinutes(5)
do {
    $ok = Invoke-Command -VMName vm-ee-test -Credential $credential -ScriptBlock {
        "ready"
    } -ErrorAction SilentlyContinue
    if ($ok -eq "ready") { break }
    Start-Sleep -Seconds 5
} while ((Get-Date) -lt $deadline)
```

Create a persistent session and copy files:

```powershell
$session = New-PSSession -VMName vm-ee-test -Credential $credential
Copy-Item -ToSession $session -Path "C:\host\payload" -Destination "C:\ee-test" -Recurse
Copy-Item -FromSession $session -Path "C:\ee-test\logs" -Destination "C:\host\logs" -Recurse
Remove-PSSession $session
```

If credentials are invalid:

- Do not keep guessing indefinitely.
- Check whether the VM is in OOBE or a login prompt through VMConnect.
- If using a prebuilt VM, treat unknown credentials as a blocker.
- Prefer rebuilding from ISO/unattend or another deterministic provisioning path.

Observed validation on the local `vm-ee-test`:

- The guest booted and KVP reported `WinDev2407Eval`.
- Common prebuilt-image credentials (`User`, `WinDev2407Eval\User`, and
  `.\User` with `Passw0rd!`) failed.
- The VHD contained a `Users\User` profile, but the unattend password data had
  been deleted.
- Therefore this VM cannot be considered a fully automated UI test target until
  it has deterministic credentials or a deterministic auto-login provisioning
  path.

## 7. Software deployment inside the guest

Use PowerShell Direct once credentials are known.

Recommended deployment steps:

1. Create `C:\ee-test` in the guest.
2. Copy `ee-win.exe`, required assets, dictionaries, and the guest-side UI test
   runner.
3. Install only the guest dependencies needed for the UI runner.
4. Launch EasyEnglish in the logged-in user desktop session.
5. Run scenarios from `Tests/UITest/scenarios/`.
6. Copy logs, screenshots, and result JSON back to the host.

Do not silently install broad development toolchains in the VM unless the test
requires building inside the guest.

## 8. UI automation rules

UI automation must run in an interactive, unlocked user desktop session.

Good options:

- A small guest-side PowerShell or .NET helper using Windows UI Automation.
- FlaUI for richer .NET-based UI checks.
- WinAppDriver later, if structured WebDriver-style desktop automation becomes
  necessary.

Avoid:

- Running the UI runner as a Windows service.
- Session 0 automation.
- Assuming VMConnect visibility means a machine-readable test runner is active.
- Disconnecting RDP in a way that locks or deactivates the desktop session.

For EasyEnglish, the first automated smoke runner should cover:

- wake flyout;
- caret visible;
- English typing accepted;
- Chinese IME composition/commit;
- Card Preview selection via keyboard and mouse;
- Chinese-to-English two-level focus;
- Bing fallback visibility.

## 9. Checkpoint lifecycle

Use checkpoints for repeatability:

```powershell
Checkpoint-VM -Name vm-ee-test -SnapshotName "CleanBase"
Checkpoint-VM -Name vm-ee-test -SnapshotName "PreUITest"
Restore-VMCheckpoint -VMName vm-ee-test -Name "PreUITest" -Confirm:$false
```

Recommended lifecycle:

1. `CleanBase`: OS, account, integration services, and guest test prerequisites
   are ready.
2. `PreUITest`: current EasyEnglish build and test runner are deployed.
3. Run tests.
4. Collect logs/screenshots.
5. Restore `PreUITest` or `CleanBase` depending on how much state the scenario
   changed.

Disable Hyper-V automatic checkpoints for this dedicated test VM so test state
is explicit and repeatable:

```powershell
Set-VM -Name vm-ee-test -AutomaticCheckpointsEnabled $false
Get-VMCheckpoint -VMName vm-ee-test |
    Where-Object { $_.Name -like "Automatic Checkpoint*" } |
    Remove-VMCheckpoint -Confirm:$false
```

Observed validation on the local `vm-ee-test`:

- Creating a validation checkpoint worked.
- Removing the validation checkpoint worked after an explicit cleanup command.
- Hyper-V had also created an automatic checkpoint; automatic checkpoints were
  disabled and the automatic checkpoint was removed.

## 10. Troubleshooting table

| Symptom | Likely cause | Action |
|---|---|---|
| Hyper-V cmdlets missing | Hyper-V disabled or reboot pending | Ask before enabling/rebooting; re-check after reboot |
| `vm-ee-test` missing | VM not created yet | Reuse existing ISO/VM package or create deterministic VM |
| No reusable ISO/package | Downloads has no suitable file | Ask/make visible before large official download |
| Official ISO API rejected | Microsoft Sentinel rejection | Stop that path; use official page/manual link or deterministic fallback |
| VM heartbeat OK but PowerShell Direct fails | Unknown/invalid guest credentials | Use VMConnect for inspection; rebuild/provision deterministic account |
| VM has IP but no login automation | Guest booted but user session unavailable | Configure auto-login through unattend/provisioning |
| WMI screenshot is black | Guest display is locked/off/security screen | Send keyboard input, use VMConnect, or ensure an unlocked auto-login session |
| `TypeCtrlAltDel()` shows security screen | Expected Hyper-V keyboard behavior | Use only for recovery; UI tests still need unlocked desktop |
| UI tests cannot find controls | Runner is in Session 0 or desktop locked | Run in interactive user session |
| VHD mount fails as in use | VM is still running or locking disk | Stop VM and confirm `Get-VM` state is `Off` |
| `cmd.exe` service bootstrap does nothing | Not a real service process | Use unattend or a proper service executable if offline recovery is required |

## 11. Final report fields

Every UI test run must report:

- VM name and state;
- Windows product/build;
- credential/provisioning method used;
- checkpoint created/restored;
- scenario markdown files executed;
- pass/fail result;
- logs/screenshots copied back to host;
- blockers or setup gaps.
