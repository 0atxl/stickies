param(
    [string]$MsysRoot,
    [string]$OutputDirectory,
    [string]$NativeNoticeRoot
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Target = "x86_64-pc-windows-gnu"

function Invoke-Checked([string]$Command, [string[]]$Arguments) {
    $Result = @(& $Command @Arguments)
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code ${LASTEXITCODE}: $Command $Arguments"
    }
    return $Result
}

function Write-Text([string]$Path, [string[]]$Lines) {
    [IO.File]::WriteAllLines($Path, $Lines, [Text.UTF8Encoding]::new($false))
}

function Write-Manifest([string]$Root, [string]$Path) {
    $Lines = @(Get-ChildItem -LiteralPath $Root -File -Recurse | Sort-Object FullName | ForEach-Object {
        $Relative = $_.FullName.Substring($Root.Length + 1).Replace('\', '/')
        "{0}  {1}" -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $Relative
    })
    Write-Text $Path $Lines
}

function Set-ZipTimestamps([string]$Root) {
    $Minimum = [datetime]::new(1980, 1, 1, 0, 0, 0)
    $Maximum = [datetime]::new(2107, 12, 31, 23, 59, 58)
    foreach ($Item in Get-ChildItem -LiteralPath $Root -Recurse -Force) {
        if ($Item.LastWriteTime -lt $Minimum -or $Item.LastWriteTime -gt $Maximum) {
            # Only staged metadata changes; original sources and file bytes stay intact.
            $Item.LastWriteTime = [datetime]::new(2000, 1, 1, 0, 0, 0)
        }
    }
}

$SavedEnvironment = @{}
foreach ($Name in @("PATH", "PKG_CONFIG", "PKG_CONFIG_PATH", "PKG_CONFIG_ALLOW_CROSS", "CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER", "LC_ALL")) {
    $SavedEnvironment[$Name] = [Environment]::GetEnvironmentVariable($Name, "Process")
}
Push-Location $RepoRoot
try {
    if ($env:OS -ne "Windows_NT" -or -not [Environment]::Is64BitProcess) { throw "Run this packaging script in 64-bit PowerShell on Windows with MSYS2 UCRT64." }
    if (-not $MsysRoot) {
        $PkgConfig = Get-Command pkg-config.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if (-not $PkgConfig) { throw "Pass -MsysRoot with the MSYS2 UCRT64 prefix, or put its pkg-config.exe on PATH." }
        $MsysRoot = Split-Path (Split-Path $PkgConfig.Source -Parent) -Parent
    }
    $MsysRoot = (Resolve-Path -LiteralPath $MsysRoot).Path.TrimEnd('\')
    if ($NativeNoticeRoot) { $NativeNoticeRoot = (Resolve-Path -LiteralPath $NativeNoticeRoot).Path.TrimEnd('\') }
    if ((Split-Path $MsysRoot -Leaf) -ne "ucrt64") { throw "Expected an MSYS2 UCRT64 prefix, got '$MsysRoot'." }
    $MsysBase = Split-Path $MsysRoot -Parent
    $Pacman = Join-Path $MsysBase "usr\bin\pacman.exe"
    $Objdump = Join-Path $MsysRoot "bin\objdump.exe"
    $Compiler = Join-Path $MsysRoot "bin\gcc.exe"
    foreach ($Tool in @($Pacman, $Objdump, $Compiler, "$MsysRoot\bin\pkg-config.exe", "$MsysRoot\bin\glib-compile-resources.exe")) {
        if (-not (Test-Path -LiteralPath $Tool -PathType Leaf)) { throw "Required build tool is missing: $Tool" }
    }
    $Cargo = (Get-Command cargo.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    $Rustc = (Get-Command rustc.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    $Git = (Get-Command git.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source

    # pkg-config's library flags need a space-free prefix. Do not assume 8.3 aliases exist.
    $Fso = New-Object -ComObject Scripting.FileSystemObject
    $ShortMsys = $Fso.GetFolder($MsysRoot).ShortPath
    if (-not $ShortMsys -or $ShortMsys.Contains(' ')) {
        throw "MSYS2 needs a space-free prefix or an existing 8.3 alias for pkg-config. Application/output paths may contain spaces."
    }
    $env:PATH = "$MsysRoot\bin;$env:PATH"
    $env:PKG_CONFIG = "$ShortMsys\bin\pkg-config.exe"
    $env:PKG_CONFIG_PATH = "$ShortMsys\lib\pkgconfig;$ShortMsys\share\pkgconfig"
    $env:PKG_CONFIG_ALLOW_CROSS = "1"
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = $Compiler
    $env:LC_ALL = "C"

    if (-not $OutputDirectory) { $OutputDirectory = Join-Path $RepoRoot "dist\windows" }
    $OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
    if (Test-Path -LiteralPath $OutputDirectory) { throw "Output already exists. Pass a new -OutputDirectory to preserve previous artifacts." }
    $SourceRoot = Join-Path $OutputDirectory "source\stickies"
    $BuildRoot = Join-Path $OutputDirectory "build"
    $DistRoot = Join-Path $OutputDirectory "stickies-windows-x86_64"
    $BinDir = Join-Path $DistRoot "bin"
    $ShareDir = Join-Path $DistRoot "share"
    $NoticeDir = Join-Path $DistRoot "licenses"
    New-Item -ItemType Directory -Path $SourceRoot, $BinDir, $ShareDir, $NoticeDir | Out-Null

    # Build the exact snapshot we distribute, including the untracked Rust modules.
    # Do not copy the repository, agent files, user data, existing ZIPs or build output.
    $Inputs = @("Cargo.toml", "Cargo.lock", "build.rs", "LICENSE", "README.md", "scripts/package-windows.ps1", "packaging/windows.md", "data/dev.stickies.Stickies.png", "data/dev.stickies.Stickies.desktop")
    foreach ($Pattern in @("src/*.rs", "assets/*.css", "assets/*.xml", "assets/fonts/*.ttf", "assets/fonts/*-LICENSE.txt", "assets/icons/*.svg", "assets/icons/edge/*.svg", "assets/icons/playful/*.svg", "migrations/*.sql")) {
        $Inputs += @(Get-ChildItem -Path (Join-Path $RepoRoot $Pattern) -File | ForEach-Object { $_.FullName.Substring($RepoRoot.Length + 1) })
    }
    foreach ($Relative in $Inputs | Sort-Object -Unique) {
        $Source = Join-Path $RepoRoot $Relative
        if ((Get-Item -LiteralPath $Source).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Source input must be a regular file: $Source" }
        $Destination = Join-Path $SourceRoot $Relative
        New-Item -ItemType Directory -Path (Split-Path $Destination -Parent) -Force | Out-Null
        Copy-Item -LiteralPath $Source -Destination $Destination
    }
    Write-Manifest $SourceRoot (Join-Path $OutputDirectory "SOURCE-MANIFEST.sha256")
    Copy-Item -LiteralPath (Join-Path $OutputDirectory "SOURCE-MANIFEST.sha256") -Destination $SourceRoot
    $Revision = (Invoke-Checked $Git @("rev-parse", "HEAD")) -join ""
    $SourceInfo = @(
        "Stickies Windows working-tree snapshot; Git baseline $Revision is not an assertion of a clean checkout.",
        "Target: $Target", "Repository: https://github.com/0atxl/stickies",
        "Matching application source: stickies-windows-x86_64-source.zip, supplied alongside the binary ZIP.",
        "SOURCE-MANIFEST.sha256 identifies the exact application inputs compiled from this snapshot.",
        "Application: MPL-2.0. Fonts and dependencies retain their original notices under licenses/.",
        "Rust sources are identified by Cargo.lock registry versions/checksums; obtain those versions from crates.io.",
        "Native package versions and upstream URLs are recorded in BUILD-INFO.txt and licenses/native/INDEX.txt.",
        "Obtain matching native source packages from https://packages.msys2.org and https://repo.msys2.org/mingw/sources/.",
        "Distributors must keep these instructions, notices and matching source available with the binary download.",
        "This records build inputs, not clean-machine compatibility or binary reproducibility."
    )
    Write-Text (Join-Path $SourceRoot "SOURCE-INFO.txt") $SourceInfo
    Copy-Item -LiteralPath (Join-Path $SourceRoot "SOURCE-INFO.txt") -Destination $DistRoot
    Copy-Item -LiteralPath (Join-Path $SourceRoot "SOURCE-MANIFEST.sha256") -Destination $DistRoot

    Push-Location $SourceRoot
    try {
        # Frozen also prevents network access. Populate the locked Cargo cache separately.
        Invoke-Checked $Cargo @("build", "--release", "--frozen", "--target", $Target, "--target-dir", $BuildRoot) | Out-Host
        $Metadata = ((Invoke-Checked $Cargo @("metadata", "--frozen", "--format-version", "1", "--filter-platform", $Target)) -join "`n") | ConvertFrom-Json
        $Tree = Invoke-Checked $Cargo @("tree", "--frozen", "--target", $Target, "-e", "normal,build", "--prefix", "none", "--format", "{p}")
    } finally { Pop-Location }
    Copy-Item -LiteralPath (Join-Path $BuildRoot "$Target\release\stickies.exe") -Destination $BinDir
    Copy-Item -LiteralPath (Join-Path $SourceRoot "LICENSE") -Destination (Join-Path $DistRoot "LICENSE")
    Copy-Item -LiteralPath (Join-Path $SourceRoot "packaging\windows.md") -Destination (Join-Path $DistRoot "README.md")
    New-Item -ItemType Directory -Path (Join-Path $NoticeDir "fonts") | Out-Null
    Copy-Item -Path (Join-Path $SourceRoot "assets\fonts\*-LICENSE.txt") -Destination (Join-Path $NoticeDir "fonts")

    $Selected = [Collections.Generic.HashSet[string]]::new()
    foreach ($Line in $Tree) {
        if ($Line -match '^(\S+) v(\S+)') { [void]$Selected.Add("$($Matches[1]) $($Matches[2])") }
    }
    $RustIndex = @("Enabled Windows Rust normal/build dependencies, including build tools. Not every entry is linked.")
    foreach ($Package in $Metadata.packages | Sort-Object -Property @("name", "version")) {
        if (-not $Package.source -or -not $Selected.Contains("$($Package.name) $($Package.version)")) { continue }
        if (-not $Package.source.StartsWith("registry+")) { throw "Dependency needs source/notice review: $($Package.name)" }
        $Crate = Split-Path $Package.manifest_path -Parent
        $Notices = @(Get-ChildItem -LiteralPath $Crate -File | Where-Object { $_.Name -match '^(license|licence|copying|notice|copyright)' })
        if ($Package.license_file) { $Notices += Get-Item -LiteralPath (Join-Path $Crate $Package.license_file) }
        if ($Notices.Count -eq 0) { throw "Missing original Rust notice: $($Package.name) $($Package.version)" }
        $Folder = Join-Path $NoticeDir "rust\$($Package.name)-$($Package.version)"
        foreach ($Notice in $Notices | Sort-Object FullName -Unique) {
            $Destination = Join-Path $Folder $Notice.FullName.Substring($Crate.Length + 1)
            New-Item -ItemType Directory -Path (Split-Path $Destination -Parent) -Force | Out-Null
            Copy-Item -LiteralPath $Notice.FullName -Destination $Destination
        }
        $RustIndex += "$($Package.name) $($Package.version) | $($Package.license) | $($Package.repository)"
    }
    Write-Text (Join-Path $NoticeDir "rust\INDEX.txt") $RustIndex

    $NativeFiles = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $GeneratedNativeFiles = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    function Copy-NativeFile([string]$Source, [string]$Destination) {
        if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw "Required runtime file is missing: $Source" }
        New-Item -ItemType Directory -Path (Split-Path $Destination -Parent) -Force | Out-Null
        Copy-Item -LiteralPath $Source -Destination $Destination
        [void]$NativeFiles.Add($Source)
    }
    function Copy-NativeTree([string]$Relative) {
        $Root = Join-Path $MsysRoot $Relative
        if (-not (Test-Path -LiteralPath $Root -PathType Container)) { throw "Required runtime directory is missing: $Root" }
        foreach ($File in Get-ChildItem -LiteralPath $Root -File -Recurse) {
            $Part = $File.FullName.Substring($MsysRoot.Length + 1)
            Copy-NativeFile $File.FullName (Join-Path $DistRoot $Part)
        }
    }
    foreach ($Helper in @("gdbus.exe", "gspawn-win64-helper.exe", "gspawn-win64-helper-console.exe")) {
        Copy-NativeFile (Join-Path $MsysRoot "bin\$Helper") (Join-Path $BinDir $Helper)
    }
    Copy-NativeTree "lib\gdk-pixbuf-2.0"
    # Cache paths must resolve against the bundle, regardless of the launch CWD.
    $LoaderRoot = Join-Path $DistRoot "lib\gdk-pixbuf-2.0"
    $Caches = @(Get-ChildItem -LiteralPath $LoaderRoot -Filter "loaders.cache" -File -Recurse)
    if ($Caches.Count -eq 0) { throw "GdkPixbuf loaders.cache is missing." }
    foreach ($Cache in $Caches) {
        # MSYS2 generates this cache from the loader DLLs; it has no package owner.
        [void]$GeneratedNativeFiles.Add((Join-Path $MsysRoot $Cache.FullName.Substring($DistRoot.Length + 1)))
        $Lines = @(Get-Content -LiteralPath $Cache.FullName | ForEach-Object {
            if ($_ -match '^"([^"\r\n]+\.dll)"$') {
                $Name = [IO.Path]::GetFileName($Matches[1].Replace('\\', '\'))
                $Module = Join-Path $Cache.DirectoryName "loaders\$Name"
                if (-not (Test-Path -LiteralPath $Module -PathType Leaf)) { throw "Loader cache refers to a missing module: $Name" }
                '"' + $Module.Substring($DistRoot.Length + 1).Replace('\', '\\') + '"'
            } elseif ($_ -match '^# LoaderDir =') {
                '# LoaderDir = bundle-relative paths'
            } else { $_ }
        })
        Write-Text $Cache.FullName $Lines
    }
    if (-not (Get-ChildItem -LiteralPath $LoaderRoot -Filter "*svg*.dll" -File -Recurse)) { throw "The SVG loader required by Stickies icons is missing." }
    foreach ($Theme in @("Adwaita", "hicolor")) {
        Copy-NativeTree "share\icons\$Theme"
        # Icon caches are generated from the copied, package-owned theme files.
        $IconCache = Join-Path $MsysRoot "share\icons\$Theme\icon-theme.cache"
        if ($NativeFiles.Contains($IconCache)) { [void]$GeneratedNativeFiles.Add($IconCache) }
    }
    $SchemaRoot = Join-Path $MsysRoot "share\glib-2.0\schemas"
    $CompiledSchemas = Join-Path $SchemaRoot "gschemas.compiled"
    Copy-NativeFile $CompiledSchemas (Join-Path $ShareDir "glib-2.0\schemas\gschemas.compiled")
    [void]$GeneratedNativeFiles.Add($CompiledSchemas)
    $SchemaInputs = @(Get-ChildItem -LiteralPath $SchemaRoot -File | Where-Object {
        $_.Name.EndsWith(".gschema.xml") -or $_.Name.EndsWith(".gschema.override")
    } | Sort-Object FullName)
    if (-not ($SchemaInputs | Where-Object { $_.Name.EndsWith(".gschema.xml") })) {
        throw "GSettings schema sources are missing; cannot identify the packages contributing to gschemas.compiled."
    }

    # These are Windows OS libraries/contracts, never arbitrary missing DLLs.
    $SystemDlls = @("advapi32.dll", "bcrypt.dll", "bcryptprimitives.dll", "cfgmgr32.dll", "comctl32.dll", "comdlg32.dll", "crypt32.dll", "d3d11.dll", "d3d12.dll", "dcomp.dll", "dnsapi.dll", "dwmapi.dll", "dwrite.dll", "dxgi.dll", "gdi32.dll", "gdiplus.dll", "hid.dll", "imm32.dll", "iphlpapi.dll", "kernel32.dll", "msimg32.dll", "ntdll.dll", "ole32.dll", "oleaut32.dll", "opengl32.dll", "rpcrt4.dll", "setupapi.dll", "shcore.dll", "shell32.dll", "shlwapi.dll", "user32.dll", "userenv.dll", "usp10.dll", "version.dll", "winmm.dll", "winspool.drv", "ws2_32.dll")
    $Queue = [Collections.Generic.Queue[string]]::new()
    $Visited = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    Get-ChildItem -LiteralPath $DistRoot -File -Recurse | Where-Object { $_.Extension -in @(".exe", ".dll") } | ForEach-Object { $Queue.Enqueue($_.FullName) }
    while ($Queue.Count -gt 0) {
        $Current = $Queue.Dequeue()
        $Headers = Invoke-Checked $Objdump @("-p", $Current)
        if (-not ($Headers -match 'file format pei-x86-64')) { throw "Expected an x86_64 PE binary: $Current" }
        foreach ($Line in $Headers) {
            if ($Line -notmatch 'DLL Name:\s+(\S+)') { continue }
            $DllName = $Matches[1]
            if (-not $Visited.Add($DllName)) { continue }
            if ($DllName -match '^(api|ext)-ms-win-[a-z0-9-]+\.dll$') { continue }
            if ($SystemDlls -contains $DllName) {
                if (-not (Test-Path -LiteralPath (Join-Path "$env:SystemRoot\System32" $DllName))) { throw "Required Windows system library is missing: $DllName" }
                continue
            }
            $SourceDll = Join-Path $MsysRoot "bin\$DllName"
            if (-not (Test-Path -LiteralPath $SourceDll -PathType Leaf)) { throw "Unresolved required dependency '$DllName' imported by '$Current'. It is neither a declared Windows library nor present in MSYS2 UCRT64." }
            $DestinationDll = Join-Path $BinDir $DllName
            Copy-NativeFile $SourceDll $DestinationDll
            $Queue.Enqueue($DestinationDll)
        }
    }

    # Generated caches have no owner. Query the bundled loaders, theme files and schema inputs instead.
    # Any other unowned native file or schema input still stops packaging.
    $NoticeInputs = @($NativeFiles | Where-Object { -not $GeneratedNativeFiles.Contains($_) }) + @($SchemaInputs.FullName)
    $NativePaths = @($NoticeInputs | Sort-Object -Unique | ForEach-Object { "/ucrt64/" + $_.Substring($MsysRoot.Length + 1).Replace('\', '/') })
    $Packages = [Collections.Generic.HashSet[string]]::new()
    for ($Offset = 0; $Offset -lt $NativePaths.Count; $Offset += 32) {
        $Last = [Math]::Min($Offset + 31, $NativePaths.Count - 1)
        foreach ($Owner in (Invoke-Checked $Pacman (@("-Qqo") + $NativePaths[$Offset..$Last]))) { [void]$Packages.Add($Owner) }
    }
    $NativeIndex = @("Original notices for every MSYS2 package owning a bundled native binary/resource or contributing schema input.")
    $PackageVersions = @()
    foreach ($Package in $Packages | Sort-Object) {
        $PackageVersions += Invoke-Checked $Pacman @("-Q", $Package)
        $NativeIndex += Invoke-Checked $Pacman @("-Qi", $Package)
        $Paths = Invoke-Checked $Pacman @("-Qlq", $Package)
        $Folder = Join-Path $NoticeDir "native\$Package"
        $Count = 0
        foreach ($Path in $Paths) {
            if ($Path -notmatch '^/ucrt64/share/licenses/(.+[^/])$') { continue }
            $Part = $Matches[1]
            $Source = Join-Path $MsysBase $Path.TrimStart('/')
            if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { continue }
            $Destination = Join-Path $Folder $Part
            New-Item -ItemType Directory -Path (Split-Path $Destination -Parent) -Force | Out-Null
            Copy-Item -LiteralPath $Source -Destination $Destination
            $Count++
        }
        if ($NativeNoticeRoot) {
            $Supplement = Join-Path $NativeNoticeRoot $Package
            if (Test-Path -LiteralPath $Supplement -PathType Container) {
                foreach ($File in Get-ChildItem -LiteralPath $Supplement -File -Recurse) {
                    $Destination = Join-Path $Folder ("supplement/" + $File.FullName.Substring($Supplement.Length + 1))
                    New-Item -ItemType Directory -Path (Split-Path $Destination -Parent) -Force | Out-Null
                    Copy-Item -LiteralPath $File.FullName -Destination $Destination
                    $Count++
                }
            }
        }
        if ($Count -eq 0) { throw "Missing original native notices for '$Package'. Supply its original notices in a '$Package' child directory under -NativeNoticeRoot; no substitute licence is generated." }
        $NativeIndex += "Notices: licenses/native/$Package", ""
    }
    Write-Text (Join-Path $NoticeDir "native\INDEX.txt") $NativeIndex
    $BuildInfo = @("UTC build time: $([DateTime]::UtcNow.ToString('o'))", "Target: $Target", "Command: cargo build --release --frozen --target $Target --target-dir <output>/build", "MSYS2 prefix: $MsysRoot", "Windows: $([Environment]::OSVersion.VersionString)")
    $BuildInfo += Invoke-Checked $Rustc @("--version", "--verbose")
    $BuildInfo += Invoke-Checked $Cargo @("--version")
    $BuildInfo += Invoke-Checked $Compiler @("--version")
    $BuildInfo += Invoke-Checked $Objdump @("--version")
    $BuildInfo += "RUSTFLAGS: $env:RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS: $env:CARGO_ENCODED_RUSTFLAGS"
    $BuildInfo += $PackageVersions
    $BuildInfo += "ZIP timestamps: staged entries outside 1980-01-01 through 2107-12-31 23:59:58 use 2000-01-01; file bytes are unchanged."
    $BuildInfo += "Generated native resources; package ownership comes from bundled loader DLLs, icon theme files and the schema inputs below:"
    $BuildInfo += @($GeneratedNativeFiles | Sort-Object | ForEach-Object { "/ucrt64/" + $_.Substring($MsysRoot.Length + 1).Replace('\', '/') })
    $BuildInfo += "GSettings compiler inputs, SHA-256 and path relative to the MSYS2 prefix:"
    foreach ($Schema in $SchemaInputs) {
        $BuildInfo += "{0}  {1}" -f (Get-FileHash -LiteralPath $Schema.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $Schema.FullName.Substring($MsysRoot.Length + 1).Replace('\', '/')
    }
    Write-Text (Join-Path $DistRoot "BUILD-INFO.txt") $BuildInfo

    $SourceZip = Join-Path $OutputDirectory "stickies-windows-x86_64-source.zip"
    $BinaryZip = Join-Path $OutputDirectory "stickies-windows-x86_64.zip"
    Set-ZipTimestamps $SourceRoot
    Compress-Archive -Path "$SourceRoot\*" -DestinationPath $SourceZip
    # Record the source archive checksum in the binary bundle before hashing it.
    $SourceChecksum = "{0}  {1}" -f (Get-FileHash -LiteralPath $SourceZip -Algorithm SHA256).Hash.ToLowerInvariant(), (Split-Path $SourceZip -Leaf)
    Write-Text (Join-Path $DistRoot "SOURCE-ARCHIVE.sha256") @($SourceChecksum)
    Write-Manifest $DistRoot (Join-Path $OutputDirectory "BUNDLE-MANIFEST.sha256")
    Copy-Item -LiteralPath (Join-Path $OutputDirectory "BUNDLE-MANIFEST.sha256") -Destination $DistRoot
    Set-ZipTimestamps $DistRoot
    Compress-Archive -Path "$DistRoot\*" -DestinationPath $BinaryZip
    $BinaryChecksum = "{0}  {1}" -f (Get-FileHash -LiteralPath $BinaryZip -Algorithm SHA256).Hash.ToLowerInvariant(), (Split-Path $BinaryZip -Leaf)
    Write-Text (Join-Path $OutputDirectory "SHA256SUMS") @($SourceChecksum, $BinaryChecksum)
    Write-Host "Distribute both ZIPs and SHA256SUMS together from $OutputDirectory"
} finally {
    Pop-Location
    foreach ($Name in $SavedEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($Name, $SavedEnvironment[$Name], "Process")
    }
}
