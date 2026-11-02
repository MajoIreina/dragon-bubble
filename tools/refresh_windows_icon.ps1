param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'

if (-not ('DragonBubble.ShellIconRefresh' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
namespace DragonBubble {
    public static class ShellIconRefresh {
        [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
        private static extern void SHChangeNotify(uint change, uint flags, string item1, IntPtr item2);
        [DllImport("shell32.dll", EntryPoint = "SHChangeNotify")]
        private static extern void InvalidateImageList(uint change, uint flags, IntPtr item1, IntPtr item2);
        public static void Refresh(string path, string directory) {
            // Open Explorer processes can retain an older image list even
            // after a file refresh. This notification invalidates that list;
            // it does not modify file associations or restart Explorer.
            InvalidateImageList(0x08000000, 0x1000, IntPtr.Zero, IntPtr.Zero);
            // Refresh the item after invalidating the list so the open view
            // requests the new image rather than repainting the old one.
            const uint pathAndFlush = 0x0005 | 0x1000;
            SHChangeNotify(0x00002000, pathAndFlush, path, IntPtr.Zero);
            SHChangeNotify(0x00001000, pathAndFlush, directory, IntPtr.Zero);
        }
    }
}
'@
}
$resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
[DragonBubble.ShellIconRefresh]::Refresh($resolvedExecutable, (Split-Path -Parent $resolvedExecutable))
Write-Output 'Explorer notified to refresh the executable icon.'
