param(
    [Parameter(Mandatory=$true)][string]$TracePath,
    [Parameter(Mandatory=$true)][string]$ReportPath
)
$ErrorActionPreference = 'Stop'
$report = @{ schema=1; status='running'; backend='isolated Windows console / VT stream / ReadConsoleOutputW'; cases=@(); restored=$false }
$console = $null
try {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

namespace MateaConsoleOracle {
    [StructLayout(LayoutKind.Sequential, Pack=2)]
    public struct Coord { public short X, Y; public Coord(short x, short y) { X=x; Y=y; } }
    [StructLayout(LayoutKind.Sequential, Pack=2)]
    public struct Rect { public short Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Explicit, Size=4)]
    public struct CharInfo {
        [FieldOffset(0)] public ushort Unicode;
        [FieldOffset(2)] public ushort Attributes;
    }
    [StructLayout(LayoutKind.Sequential, Pack=2)]
    public struct BufferInfo {
        public Coord Size, Cursor;
        public ushort Attributes;
        public Rect Window;
        public Coord MaximumWindow;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct CursorInfo { public uint Size; public int Visible; }

    public sealed class ConsoleProbe : IDisposable {
        [DllImport("kernel32.dll", CharSet=CharSet.Unicode, ExactSpelling=true, SetLastError=true)]
        static extern SafeFileHandle CreateFileW(string name, uint access, uint sharing,
            IntPtr security, uint creation, uint flags, IntPtr template);
        [DllImport("kernel32.dll", SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool GetConsoleMode(SafeFileHandle handle, out uint mode);
        [DllImport("kernel32.dll", SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool SetConsoleMode(SafeFileHandle handle, uint mode);
        [DllImport("kernel32.dll", SetLastError=true)]
        static extern uint GetConsoleOutputCP();
        [DllImport("kernel32.dll", SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool SetConsoleOutputCP(uint codePage);
        [DllImport("kernel32.dll", SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool GetConsoleScreenBufferInfo(SafeFileHandle handle, out BufferInfo info);
        [DllImport("kernel32.dll", SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool GetConsoleCursorInfo(SafeFileHandle handle, out CursorInfo info);
        [DllImport("kernel32.dll", SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool SetConsoleCursorInfo(SafeFileHandle handle, ref CursorInfo info);
        [DllImport("kernel32.dll", SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool SetConsoleTextAttribute(SafeFileHandle handle, ushort attributes);
        [DllImport("kernel32.dll", SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool SetConsoleCursorPosition(SafeFileHandle handle, Coord position);
        [DllImport("kernel32.dll", SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool WriteFile(SafeFileHandle handle, byte[] bytes, uint count,
            out uint written, IntPtr overlapped);
        [DllImport("kernel32.dll", SetLastError=true, EntryPoint="ReadConsoleOutputW", ExactSpelling=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool ReadConsoleOutput(SafeFileHandle handle, [Out] CharInfo[] cells,
            Coord size, Coord origin, ref Rect region);

        SafeFileHandle handle;
        uint mode, codePage;
        BufferInfo original;
        CursorInfo cursor;
        bool modeSaved, codePageSaved, infoSaved, cursorSaved, disposed;
        public bool Restored { get; private set; }
        public uint ModeBefore { get { return mode; } }
        public uint CodePageBefore { get { return codePage; } }
        public uint ModeAfter { get; private set; }
        public uint CodePageAfter { get; private set; }
        public int WindowWidth { get { return original.Window.Right-original.Window.Left+1; } }

        static void Check(bool value, string operation) {
            if (!value) throw new Win32Exception(Marshal.GetLastWin32Error(), operation);
        }
        public ConsoleProbe() {
            if (Marshal.SizeOf(typeof(Coord))!=4 || Marshal.SizeOf(typeof(Rect))!=8 ||
                Marshal.SizeOf(typeof(CharInfo))!=4 || Marshal.SizeOf(typeof(BufferInfo))!=22 ||
                Marshal.SizeOf(typeof(CursorInfo))!=8)
                throw new InvalidOperationException("Native console structure layout mismatch");
            handle=CreateFileW("CONOUT$",0xC0000000,3,IntPtr.Zero,3,0x80,IntPtr.Zero);
            if (handle.IsInvalid) { handle.Dispose(); throw new Win32Exception(Marshal.GetLastWin32Error(),"CreateFileW(CONOUT$)"); }
            try {
                Check(GetConsoleMode(handle,out mode),"GetConsoleMode"); modeSaved=true;
                codePage=GetConsoleOutputCP(); Check(codePage!=0,"GetConsoleOutputCP"); codePageSaved=true;
                Check(GetConsoleScreenBufferInfo(handle,out original),"GetConsoleScreenBufferInfo"); infoSaved=true;
                Check(GetConsoleCursorInfo(handle,out cursor),"GetConsoleCursorInfo"); cursorSaved=true;
                Check(SetConsoleMode(handle,mode|5),"Enable processed VT output");
                Check(SetConsoleOutputCP(65001),"Set UTF-8 output code page");
            } catch { Dispose(); throw; }
        }
        public string WriteAndRead(string stream, short width) {
            if (disposed || width<=0 || width>WindowWidth) throw new InvalidOperationException("Invalid console read geometry");
            byte[] bytes=Encoding.UTF8.GetBytes(stream);
            int offset=0;
            while (offset<bytes.Length) {
                byte[] tail=new byte[bytes.Length-offset]; Array.Copy(bytes,offset,tail,0,tail.Length);
                uint written; Check(WriteFile(handle,tail,(uint)tail.Length,out written,IntPtr.Zero),"WriteFile(VT frame)");
                if (written==0 || written>(uint)tail.Length) throw new InvalidOperationException("Invalid console write count");
                offset+=(int)written;
            }
            BufferInfo info; Check(GetConsoleScreenBufferInfo(handle,out info),"Read native viewport origin");
            Rect region=new Rect { Left=info.Window.Left, Top=info.Window.Top,
                Right=(short)(info.Window.Left+width-1), Bottom=info.Window.Top };
            Rect requested=region;
            CharInfo[] cells=new CharInfo[width];
            Check(ReadConsoleOutput(handle,cells,new Coord(width,1),new Coord(0,0),ref region),"ReadConsoleOutputW");
            if (region.Left!=requested.Left || region.Top!=requested.Top || region.Right!=requested.Right || region.Bottom!=requested.Bottom)
                throw new InvalidOperationException("Native console clipped the requested observation");
            StringBuilder text=new StringBuilder(width);
            foreach (CharInfo cell in cells) text.Append((char)cell.Unicode);
            return text.ToString();
        }
        public void Dispose() {
            if (disposed) return;
            disposed=true;
            bool good=true;
            try {
                if (cursorSaved) good=SetConsoleCursorInfo(handle,ref cursor)&&good;
                if (infoSaved) good=SetConsoleTextAttribute(handle,original.Attributes)&&good;
                if (infoSaved) good=SetConsoleCursorPosition(handle,original.Cursor)&&good;
                if (codePageSaved) good=SetConsoleOutputCP(codePage)&&good;
                if (modeSaved) good=SetConsoleMode(handle,mode)&&good;
                uint restoredMode=0;
                if (modeSaved) good=GetConsoleMode(handle,out restoredMode)&&restoredMode==mode&&good;
                ModeAfter=restoredMode; CodePageAfter=GetConsoleOutputCP();
                if (codePageSaved) good=CodePageAfter==codePage&&good;
                if (cursorSaved) {
                    CursorInfo after; good=GetConsoleCursorInfo(handle,out after)&&after.Size==cursor.Size&&after.Visible==cursor.Visible&&good;
                }
                if (infoSaved) {
                    BufferInfo after; good=GetConsoleScreenBufferInfo(handle,out after)&&after.Attributes==original.Attributes&&
                        after.Cursor.X==original.Cursor.X&&after.Cursor.Y==original.Cursor.Y&&good;
                }
                Restored=good;
            } finally { handle.Dispose(); }
            if (!good) throw new InvalidOperationException("Console state restoration failed");
        }
    }
}
'@
    $trace = [IO.File]::ReadAllText($TracePath,[Text.Encoding]::UTF8) | ConvertFrom-Json
    if ($trace.schema -ne 1 -or $trace.kind -cne 'native-windows-console-input' -or $trace.frames.Count -ne 36 -or $trace.read_width -ne 60) {
        throw 'Unexpected native console input schema'
    }
    $console = [MateaConsoleOracle.ConsoleProbe]::new()
    $report.window_width = $console.WindowWidth
    $report.mode_before = $console.ModeBefore
    $report.code_page_before = $console.CodePageBefore
    $report.powershell = $PSVersionTable.PSVersion.ToString()
    $report.os = [Environment]::OSVersion.VersionString
    foreach ($frame in $trace.frames) {
        $expected = ([string]$frame.expected).PadRight(60,' ')
        $actual = $console.WriteAndRead([string]$frame.ansi,60)
        if (-not [string]::Equals($actual,$expected,[StringComparison]::Ordinal)) {
            $report.mismatch = @{case=$frame.case; expected=$expected; actual=$actual}
            throw "Native console cells differ: $($frame.case)"
        }
        $report.cases += @{case=$frame.case; width=$frame.width; exact_cells=60; observed=$actual}
    }
    $report.status = 'complete'
} catch {
    $report.status = 'failed'
    $report.error = $_.Exception.Message
} finally {
    if ($null -ne $console) {
        try { $console.Dispose() } catch { $report.status='failed'; $report.cleanup_error=$_.Exception.Message }
        $report.restored=$console.Restored
        $report.mode_after=$console.ModeAfter
        $report.code_page_after=$console.CodePageAfter
    }
    $json=$report | ConvertTo-Json -Depth 10
    [IO.File]::WriteAllText($ReportPath,$json,(New-Object Text.UTF8Encoding($false)))
}
if ($report.status -ne 'complete' -or -not $report.restored) { exit 1 }
