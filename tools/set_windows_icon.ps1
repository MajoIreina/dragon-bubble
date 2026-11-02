param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$IconPath
)
$ErrorActionPreference = 'Stop'

# The preview launcher is the official Godot GUI binary, whose native icon
# group is GODOT_ICON. Replace that group's images without changing its code.
if (-not ('DragonBubble.WindowsIcon' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.IO;
using System.Runtime.InteropServices;

namespace DragonBubble {
    public static class WindowsIcon {
        private delegate bool LanguageCallback(IntPtr module, IntPtr type, IntPtr name, ushort language, IntPtr parameter);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr LoadLibraryExW(string path, IntPtr file, uint flags);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool FreeLibrary(IntPtr module);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern bool EnumResourceLanguagesW(IntPtr module, IntPtr type, string name, LanguageCallback callback, IntPtr parameter);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr BeginUpdateResourceW(string path, bool deleteExisting);
        [DllImport("kernel32.dll", EntryPoint = "UpdateResourceW", SetLastError = true)]
        private static extern bool UpdateImage(IntPtr update, IntPtr type, IntPtr name, ushort language, byte[] data, uint size);
        [DllImport("kernel32.dll", EntryPoint = "UpdateResourceW", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern bool UpdateGroup(IntPtr update, IntPtr type, string name, ushort language, byte[] data, uint size);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool EndUpdateResourceW(IntPtr update, bool discard);

        public static void Apply(string executable, string iconPath) {
            byte[] icon = File.ReadAllBytes(iconPath);
            if (icon.Length < 6 || BitConverter.ToUInt16(icon, 0) != 0 || BitConverter.ToUInt16(icon, 2) != 1)
                throw new InvalidDataException("Invalid ICO header");
            int count = BitConverter.ToUInt16(icon, 4);
            if (count == 0 || count > 256 || icon.Length < 6 + 16 * count)
                throw new InvalidDataException("Invalid ICO image directory");
            byte[][] images = new byte[count][];
            byte[] group;
            using (var stream = new MemoryStream()) {
                using (var writer = new BinaryWriter(stream)) {
                    writer.Write((ushort)0);
                    writer.Write((ushort)1);
                    writer.Write((ushort)count);
                    for (int i = 0; i < count; i++) {
                        int entry = 6 + 16 * i;
                        uint size = BitConverter.ToUInt32(icon, entry + 8);
                        uint offset = BitConverter.ToUInt32(icon, entry + 12);
                        if (size == 0 || offset < 6 + 16 * count || (ulong)offset + size > (ulong)icon.Length)
                            throw new InvalidDataException("ICO image is outside the file");
                        images[i] = new byte[checked((int)size)];
                        Buffer.BlockCopy(icon, checked((int)offset), images[i], 0, images[i].Length);
                        writer.Write(icon, entry, 12);
                        writer.Write((ushort)(i + 1));
                    }
                    group = stream.ToArray();
                }
            }
            var languages = new List<ushort>();
            IntPtr module = LoadLibraryExW(executable, IntPtr.Zero, 2);
            if (module == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
            try {
                LanguageCallback collect = delegate(IntPtr m, IntPtr t, IntPtr n, ushort language, IntPtr p) {
                    languages.Add(language);
                    return true;
                };
                if (!EnumResourceLanguagesW(module, new IntPtr(14), "GODOT_ICON", collect, IntPtr.Zero))
                    throw new Win32Exception(Marshal.GetLastWin32Error(), "The Godot launcher icon group could not be read");
                GC.KeepAlive(collect);
            } finally {
                FreeLibrary(module);
            }
            IntPtr update = BeginUpdateResourceW(executable, false);
            if (update == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
            try {
                foreach (ushort language in languages) {
                    for (int i = 0; i < count; i++) {
                        if (!UpdateImage(update, new IntPtr(3), new IntPtr(i + 1), language, images[i], (uint)images[i].Length))
                            throw new Win32Exception(Marshal.GetLastWin32Error());
                    }
                    if (!UpdateGroup(update, new IntPtr(14), "GODOT_ICON", language, group, (uint)group.Length))
                        throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                IntPtr commit = update;
                update = IntPtr.Zero;
                if (!EndUpdateResourceW(commit, false)) throw new Win32Exception(Marshal.GetLastWin32Error());
            } finally {
                if (update != IntPtr.Zero) EndUpdateResourceW(update, true);
            }
        }
    }
}
'@
}
[DragonBubble.WindowsIcon]::Apply((Resolve-Path -LiteralPath $Executable).Path, (Resolve-Path -LiteralPath $IconPath).Path)
Write-Output 'Windows executable icon synchronized with Settings.'
