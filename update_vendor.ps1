# update_vendor.ps1
$SCRIPT_DIR = Split-Path -Parent $MyInvocation.MyCommand.Path
$VENDOR_DIR = Join-Path $SCRIPT_DIR 'vendor'
$DEPS_DIR   = Join-Path $VENDOR_DIR 'deps'

$CORE_REPOS = @(
    @{ Name = 'kyutil';  Url = 'https://gitlab.com/kyber/core/kyutil.git'  }
    @{ Name = 'kymux';   Url = 'https://gitlab.com/kyber/core/kymux.git'   }
    @{ Name = 'kymedia'; Url = 'https://gitlab.com/kyber/core/kymedia.git' }
    @{ Name = 'kynput';  Url = 'https://gitlab.com/kyber/core/kynput.git'  }
    @{ Name = 'kysdk';   Url = 'https://gitlab.com/kyber/core/kysdk.git'   }
)

$DEPS_REPOS = @(
    @{ Name = 'keycode';      Url = 'https://gitlab.com/kyber/deps/keycode.git'      }
    @{ Name = 'libudev-sys';  Url = 'https://gitlab.com/kyber/deps/libudev-sys.git'  }
    @{ Name = 'libvlcjni';    Url = 'https://gitlab.com/kyber/deps/libvlcjni.git'    }
    @{ Name = 'rust-sdl2';    Url = 'https://gitlab.com/kyber/deps/rust-sdl2.git'    }
    @{ Name = 'txproto';      Url = 'https://gitlab.com/kyber/deps/txproto.git'      }
    @{ Name = 'vigem-client'; Url = 'https://gitlab.com/kyber/deps/vigem-client.git' }
    @{ Name = 'vlc';          Url = 'https://gitlab.com/kyber/deps/vlc.git'          }
    @{ Name = 'vlc-rs';       Url = 'https://gitlab.com/kyber/deps/vlc-rs.git'       }
    @{ Name = 'winit';        Url = 'https://gitlab.com/kyber/deps/winit.git'        }
)

New-Item -ItemType Directory -Path $VENDOR_DIR -Force | Out-Null
New-Item -ItemType Directory -Path $DEPS_DIR   -Force | Out-Null

function Sync-Repo {
    param([string]$Name, [string]$Url, [string]$Dest)

    if (Test-Path (Join-Path $Dest '.git')) {
        Write-Host "pull $Name"
        git -C $Dest pull --ff-only
    } else {
        Write-Host "clone $Name"
        git clone --depth=1 $Url $Dest
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERREUR $Name code $LASTEXITCODE"
    }
}

Write-Host "--- CORE ---"
foreach ($r in $CORE_REPOS) {
    Sync-Repo -Name $r.Name -Url $r.Url -Dest (Join-Path $VENDOR_DIR $r.Name)
}

Write-Host "--- DEPS ---"
foreach ($r in $DEPS_REPOS) {
    Sync-Repo -Name $r.Name -Url $r.Url -Dest (Join-Path $DEPS_DIR $r.Name)
}

Write-Host "done"