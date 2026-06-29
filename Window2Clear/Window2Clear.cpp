#include <windows.h>
#include <shellapi.h>
#include <commctrl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "resource.h"

#pragma comment(lib, "user32.lib")
#pragma comment(lib, "shell32.lib")
#pragma comment(lib, "comctl32.lib")

// 调试日志（生产环境关闭）
//#define ENABLE_DEBUG_LOG
#ifdef ENABLE_DEBUG_LOG
#define DEBUG_LOG_FILE L".\\debug.log"
void DebugLog(const wchar_t* fmt, ...) {
    va_list args;
    va_start(args, fmt);
    wchar_t buf[1024];
    vswprintf(buf, 1024, fmt, args);
    va_end(args);
    FILE* f = _wfopen(DEBUG_LOG_FILE, L"a");
    if (f) {
        SYSTEMTIME st;
        GetLocalTime(&st);
        fwprintf(f, L"[%02d:%02d:%02d.%03d] %s\n",
            st.wHour, st.wMinute, st.wSecond, st.wMilliseconds, buf);
        fclose(f);
    }
}
#else
#define DebugLog(...)
#endif

// 常量定义
#define WM_TRAYICON (WM_USER + 1)
#define ID_TRAY_EXIT 1001
#define ID_TRAY_SETTINGS 1002
#define ID_HOTKEY_TRANSPARENCY_UP 1
#define ID_HOTKEY_TRANSPARENCY_DOWN 2
#define ID_HOTKEY_CENTER_WINDOW 3
#define ID_HOTKEY_SHAKE_WINDOW 4
#define ID_HOTKEY_RESTORE_OPACITY 5
#define MAX_PATH_LEN 260
#define APP_VERSION L"v0.3.0"
// 配置文件路径，在 WinMain 中基于 exe 目录动态生成
wchar_t g_configFile[MAX_PATH_LEN] = L".\\config.ini";

// 控件ID
#define IDC_TRANSPARENCY_UP_BUTTON 2001
#define IDC_TRANSPARENCY_DOWN_BUTTON 2002
#define IDC_CENTER_BUTTON 2003
#define IDC_SHAKE_BUTTON 2004
#define IDC_TRANSPARENCY_SLIDER 2005
#define IDC_SAVE_BUTTON 2006
#define IDC_TRANSPARENCY_LABEL 2007
#define IDC_TRANSPARENCY_UP_DISPLAY 2008
#define IDC_TRANSPARENCY_DOWN_DISPLAY 2009
#define IDC_CENTER_DISPLAY 2010
#define IDC_SHAKE_DISPLAY 2011
#define IDC_TRANSPARENCY_ENABLE 2012
#define IDC_CENTER_ENABLE 2013
#define IDC_SHAKE_ENABLE 2014
#define IDC_RESTORE_DISPLAY 2015
#define IDC_RESTORE_BUTTON 2016
#define IDC_RESTORE_ENABLE 2017

// 全局变量
HWND g_hMainWnd = NULL;           // 主窗口句柄
HWND g_hSettingsWnd = NULL;       // 设置窗口句柄
NOTIFYICONDATA g_nid = { 0 };     // 系统托盘图标数据
int g_transparencyStep = 10;      // 透明度调整步长（默认10%）

// 透明度热键设置
UINT g_transparencyUpModifiers = MOD_ALT;    // 透明度增加修饰键
UINT g_transparencyUpKey = VK_LEFT;          // 透明度增加按键
UINT g_transparencyDownModifiers = MOD_ALT;  // 透明度减少修饰键
UINT g_transparencyDownKey = VK_RIGHT;       // 透明度减少按键
BOOL g_enableTransparencyUp = TRUE;          // 是否启用透明度增加功能
BOOL g_enableTransparencyDown = TRUE;        // 是否启用透明度减少功能

// 窗口居中热键设置
UINT g_centerModifiers = MOD_CONTROL;        // 窗口居中修饰键
UINT g_centerKey = VK_NUMPAD5;               // 窗口居中按键
BOOL g_enableCenter = FALSE;                 // 是否启用窗口居中功能

// 窗口抖动热键设置
UINT g_shakeModifiers = MOD_ALT;             // 窗口抖动修饰键
UINT g_shakeKey = VK_DOWN;                   // 窗口抖动按键
BOOL g_enableShake = FALSE;                  // 是否启用窗口抖动功能

// 恢复透明度热键设置
UINT g_restoreModifiers = MOD_ALT;           // 恢复透明度修饰键
UINT g_restoreKey = VK_UP;                   // 恢复透明度按键
BOOL g_enableRestore = TRUE;                 // 是否启用恢复透明度功能

// 透明窗口持久化
#define MAX_TRANSPARENT_WINDOWS 64

typedef struct {
    HWND hwnd;
    int alpha;
} TransparentWindow;

TransparentWindow g_transparentWindows[MAX_TRANSPARENT_WINDOWS];
int g_transparentCount = 0;

// 窗口抖动状态
#define SHAKE_TIMER_ID 3
#define SHAKE_STEPS 6
#define SHAKE_DISTANCE 5
static HWND g_shakeHwnd = NULL;
static int g_shakeOrigX = 0;
static int g_shakeOrigY = 0;
static int g_shakeStep = 0;

// 热键监听状态
BOOL g_isListeningHotkey = FALSE;     // 是否正在监听热键输入
int g_currentListeningType = 0;       // 当前监听类型: 0=无, 1=透明度增加, 2=透明度减少, 3=居中, 4=抖动
HWND g_hCurrentButton = NULL;         // 当前正在设置的按钮句柄
HWND g_hCurrentDisplay = NULL;        // 当前正在设置的显示框句柄
DWORD g_listeningStartTime = 0;       // 监听开始时间（用于超时检测）

// 函数声明
LRESULT CALLBACK WindowProc(HWND hwnd, UINT uMsg, WPARAM wParam, LPARAM lParam);         // 主窗口消息处理函数
LRESULT CALLBACK SettingsProc(HWND hwnd, UINT uMsg, WPARAM wParam, LPARAM lParam);       // 设置窗口消息处理函数
void CreateTrayIcon(HWND hwnd);                   // 创建系统托盘图标
void RemoveTrayIcon();                            // 移除系统托盘图标
void ShowContextMenu(HWND hwnd);                  // 显示右键菜单
void ShowSettingsWindow();                        // 显示设置窗口
void RegisterHotKeys(HWND hwnd);                  // 注册全局热键
void UnregisterHotKeys(HWND hwnd);                // 注销全局热键
void AdjustWindowTransparency(BOOL increase);     // 调整窗口透明度
void CenterWindow();                              // 将窗口居中显示
void ShakeWindow();                               // 窗口抖动效果
void RestoreWindowOpacity();                      // 恢复窗口不透明
int FindTransparentWindow(HWND hwnd);             // 查找已跟踪的透明窗口
void TrackTransparentWindow(HWND hwnd, int alpha);// 跟踪透明窗口
void UntrackTransparentWindow(HWND hwnd);         // 取消跟踪透明窗口
void LoadConfig();                                // 从配置文件加载设置
void SaveConfig();                                // 保存设置到配置文件
HWND GetTopMostWindow();                          // 获取最上层窗口
void SetWindowTransparency(HWND hwnd, int alpha); // 设置窗口透明度
int GetWindowTransparency(HWND hwnd);             // 获取窗口透明度
void GetModifierName(UINT modifiers, wchar_t* buf, size_t bufSize);         // 获取修饰键名称
void GetKeyName(UINT vkCode, wchar_t* buf, size_t bufSize);                 // 获取按键名称

// 主函数
int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow)
{
    // 初始化通用控件
    InitCommonControls();

    // 检查是否已有实例运行
    HWND existingWnd = FindWindow(L"Window2ClearClass", NULL);
    if (existingWnd) {
        MessageBox(NULL, L"Window2Clear 已在运行中！", L"提示", MB_OK | MB_ICONINFORMATION);
        return 0;
    }

    // 注册窗口类
    WNDCLASS wc = { 0 };
    wc.lpfnWndProc = WindowProc;
    wc.hInstance = hInstance;
    wc.lpszClassName = L"Window2ClearClass";
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    wc.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);
    wc.hIcon = LoadIcon(hInstance, MAKEINTRESOURCE(IDI_WINDOW2CLEAR));

    if (!RegisterClass(&wc)) {
        MessageBox(NULL, L"注册窗口类失败！", L"错误", MB_OK | MB_ICONERROR);
        return 1;
    }

    // 初始化配置文件路径（基于 exe 所在目录）
    {
        wchar_t exePath[MAX_PATH_LEN];
        GetModuleFileNameW(NULL, exePath, MAX_PATH_LEN);
        // 去掉文件名，保留目录
        wchar_t* lastSlash = wcsrchr(exePath, L'\\');
        if (lastSlash) {
            *(lastSlash + 1) = L'\0';
            swprintf_s(g_configFile, MAX_PATH_LEN, L"%sconfig.ini", exePath);
        }
    }

    // 加载配置
    LoadConfig();

    // 创建隐藏的主窗口
    g_hMainWnd = CreateWindow(
        L"Window2ClearClass",
        L"Window2Clear",
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT, CW_USEDEFAULT,
        400, 300,
        NULL, NULL, hInstance, NULL
    );

    if (!g_hMainWnd) {
        MessageBox(NULL, L"创建主窗口失败！", L"错误", MB_OK | MB_ICONERROR);
        return 1;
    }

    // 创建系统托盘图标
    CreateTrayIcon(g_hMainWnd);

    // 注册全局热键
    RegisterHotKeys(g_hMainWnd);

    // 显示启动提示
    wchar_t startupMsg[500];
    swprintf_s(startupMsg, 500, L"Window2Clear %s 已启动！\n\n默认热键：\n- Alt+←/→ 调整窗口透明度\n- Alt+↑ 恢复窗口不透明\n- Ctrl+数字键5 窗口居中（需开启）\n- Alt+↓ 窗口抖动（需开启）\n\n右键点击托盘图标进行设置", APP_VERSION);
    MessageBox(NULL, startupMsg, L"Window2Clear 启动成功", MB_OK | MB_ICONINFORMATION);

    // 消息循环
    MSG msg;
    while (GetMessage(&msg, NULL, 0, 0)) {
        TranslateMessage(&msg);
        DispatchMessage(&msg);
    }

    // 清理资源
    UnregisterHotKeys(g_hMainWnd);
    RemoveTrayIcon();

    return (int)msg.wParam;
}

// 主窗口过程
LRESULT CALLBACK WindowProc(HWND hwnd, UINT uMsg, WPARAM wParam, LPARAM lParam)
{
    switch (uMsg) {
    case WM_CREATE:
        // 10ms 定时器，快速检测并恢复
        SetTimer(hwnd, 1, 10, NULL);
        break;

    case WM_TIMER:
        if (wParam == 1) {
            for (int i = g_transparentCount - 1; i >= 0; i--) {
                HWND twHwnd = g_transparentWindows[i].hwnd;
                if (!IsWindow(twHwnd)) {
                    UntrackTransparentWindow(twHwnd);
                    continue;
                }

                LONG es = GetWindowLong(twHwnd, GWL_EXSTYLE);
                BOOL layered = (es & WS_EX_LAYERED) != 0;

                // 读当前 alpha
                BYTE curAlpha = 255;
                COLORREF colorKey;
                DWORD flags;
                BOOL gotAlpha = FALSE;
                if (layered) {
                    gotAlpha = GetLayeredWindowAttributes(twHwnd, &colorKey, &curAlpha, &flags);
                }

                BYTE wantAlpha = (BYTE)g_transparentWindows[i].alpha;

                // 只在确实需要时才写
                if (!layered) {
                    // WS_EX_LAYERED 被清了 → 必须重设
                    SetWindowLong(twHwnd, GWL_EXSTYLE, es | WS_EX_LAYERED);
                    SetLayeredWindowAttributes(twHwnd, 0, wantAlpha, LWA_ALPHA);
                }
                else if (gotAlpha && curAlpha != wantAlpha) {
                    // alpha 被改了 → 只重设 alpha，不碰 style
                    SetLayeredWindowAttributes(twHwnd, 0, wantAlpha, LWA_ALPHA);
                }
                // else: 一切正常，什么都不做
            }
        }
        if (wParam == SHAKE_TIMER_ID) {
            if (g_shakeStep < SHAKE_STEPS) {
                int dx = (g_shakeStep % 2 == 0) ? SHAKE_DISTANCE : -SHAKE_DISTANCE;
                int dy = (g_shakeStep % 4 < 2) ? SHAKE_DISTANCE : -SHAKE_DISTANCE;
                if (IsWindow(g_shakeHwnd)) {
                    SetWindowPos(g_shakeHwnd, NULL, g_shakeOrigX + dx, g_shakeOrigY + dy, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
                }
                g_shakeStep++;
            } else {
                KillTimer(hwnd, SHAKE_TIMER_ID);
                if (IsWindow(g_shakeHwnd)) {
                    SetWindowPos(g_shakeHwnd, NULL, g_shakeOrigX, g_shakeOrigY, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
                }
                g_shakeHwnd = NULL;
            }
        }
        break;

    case WM_HOTKEY:
        switch (wParam) {
        case ID_HOTKEY_TRANSPARENCY_UP:
            AdjustWindowTransparency(TRUE); // 增加透明度（减少不透明度）
            break;
        case ID_HOTKEY_TRANSPARENCY_DOWN:
            AdjustWindowTransparency(FALSE); // 减少透明度（增加不透明度）
            break;
        case ID_HOTKEY_CENTER_WINDOW:
            CenterWindow(); // 窗口居中
            break;
        case ID_HOTKEY_SHAKE_WINDOW:
            ShakeWindow(); // 窗口抖动
            break;
        case ID_HOTKEY_RESTORE_OPACITY:
            RestoreWindowOpacity(); // 恢复窗口不透明
            break;
        }
        break;

    case WM_TRAYICON:
        if (lParam == WM_RBUTTONUP) {
            ShowContextMenu(hwnd);
        } else if (lParam == WM_LBUTTONDBLCLK) {
            ShowSettingsWindow();
        }
        break;

    case WM_COMMAND:
        switch (LOWORD(wParam)) {
        case ID_TRAY_SETTINGS:
            ShowSettingsWindow();
            break;
        case ID_TRAY_EXIT:
            PostQuitMessage(0);
            break;
        }
        break;

    case WM_DESTROY:
        KillTimer(hwnd, 1);
        KillTimer(hwnd, SHAKE_TIMER_ID);
        PostQuitMessage(0);
        break;

    default:
        return DefWindowProc(hwnd, uMsg, wParam, lParam);
    }
    return 0;
}

// 设置窗口过程
LRESULT CALLBACK SettingsProc(HWND hwnd, UINT uMsg, WPARAM wParam, LPARAM lParam)
{
    static HWND hTransparencySlider, hSaveButton, hTransparencyLabel;
    static HWND hTransparencyUpDisplay, hTransparencyDownDisplay, hCenterDisplay, hShakeDisplay;
    static HWND hTransparencyUpButton, hTransparencyDownButton, hCenterButton, hShakeButton;
    static HWND hTransparencyEnable, hCenterEnable, hShakeEnable, hRestoreEnable;
    static HWND hRestoreDisplay, hRestoreButton;

    switch (uMsg) {
    case WM_CREATE:
    {
        // 打开设置窗口时，先取消所有热键监听
        UnregisterHotKeys(g_hMainWnd);

        int yPos = 20;

        wchar_t _modBuf[64], _keyBuf[32];  // reused for hotkey display

        // 透明度功能区
        CreateWindow(L"STATIC", L"透明度控制:",
            WS_VISIBLE | WS_CHILD,
            20, yPos, 120, 20,
            hwnd, NULL, GetModuleHandle(NULL), NULL);

        hTransparencyEnable = CreateWindow(L"BUTTON", L"启用",
            WS_VISIBLE | WS_CHILD | BS_AUTOCHECKBOX,
            150, yPos, 60, 20,
            hwnd, (HMENU)IDC_TRANSPARENCY_ENABLE, GetModuleHandle(NULL), NULL);
        SendMessage(hTransparencyEnable, BM_SETCHECK, (g_enableTransparencyUp || g_enableTransparencyDown) ? BST_CHECKED : BST_UNCHECKED, 0);

        yPos += 30;

        // 透明度增加热键
        CreateWindow(L"STATIC", L"增加透明度:",
            WS_VISIBLE | WS_CHILD,
            30, yPos, 80, 20,
            hwnd, NULL, GetModuleHandle(NULL), NULL);

        hTransparencyUpDisplay = CreateWindow(L"EDIT", L"",
            WS_VISIBLE | WS_CHILD | WS_BORDER | ES_AUTOHSCROLL | ES_READONLY,
            120, yPos, 140, 20,
            hwnd, (HMENU)IDC_TRANSPARENCY_UP_DISPLAY, GetModuleHandle(NULL), NULL);
        // 设置当前热键显示
        wchar_t keyText[256];
        GetModifierName(g_transparencyUpModifiers, _modBuf, 64);
        GetKeyName(g_transparencyUpKey, _keyBuf, 32);
        swprintf(keyText, 256, L"%s+%s", _modBuf, _keyBuf);
        SetWindowText(hTransparencyUpDisplay, keyText);

        hTransparencyUpButton = CreateWindow(L"BUTTON", L"设置",
            WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON,
            270, yPos, 50, 20,
            hwnd, (HMENU)IDC_TRANSPARENCY_UP_BUTTON, GetModuleHandle(NULL), NULL);

        yPos += 30;

        // 透明度减少热键
        CreateWindow(L"STATIC", L"减少透明度:",
            WS_VISIBLE | WS_CHILD,
            30, yPos, 80, 20,
            hwnd, NULL, GetModuleHandle(NULL), NULL);

        hTransparencyDownDisplay = CreateWindow(L"EDIT", L"",
            WS_VISIBLE | WS_CHILD | WS_BORDER | ES_AUTOHSCROLL | ES_READONLY,
            120, yPos, 140, 20,
            hwnd, (HMENU)IDC_TRANSPARENCY_DOWN_DISPLAY, GetModuleHandle(NULL), NULL);
        // 设置当前热键显示
        GetModifierName(g_transparencyDownModifiers, _modBuf, 64);
        GetKeyName(g_transparencyDownKey, _keyBuf, 32);
        swprintf(keyText, 256, L"%s+%s", _modBuf, _keyBuf);
        SetWindowText(hTransparencyDownDisplay, keyText);

        hTransparencyDownButton = CreateWindow(L"BUTTON", L"设置",
            WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON,
            270, yPos, 50, 20,
            hwnd, (HMENU)IDC_TRANSPARENCY_DOWN_BUTTON, GetModuleHandle(NULL), NULL);

        yPos += 40;

        // 窗口居中功能区
        CreateWindow(L"STATIC", L"窗口居中:",
            WS_VISIBLE | WS_CHILD,
            20, yPos, 80, 20,
            hwnd, NULL, GetModuleHandle(NULL), NULL);

        hCenterEnable = CreateWindow(L"BUTTON", L"启用",
            WS_VISIBLE | WS_CHILD | BS_AUTOCHECKBOX,
            150, yPos, 60, 20,
            hwnd, (HMENU)IDC_CENTER_ENABLE, GetModuleHandle(NULL), NULL);
        SendMessage(hCenterEnable, BM_SETCHECK, g_enableCenter ? BST_CHECKED : BST_UNCHECKED, 0);

        yPos += 30;

        hCenterDisplay = CreateWindow(L"EDIT", L"",
            WS_VISIBLE | WS_CHILD | WS_BORDER | ES_AUTOHSCROLL | ES_READONLY,
            30, yPos, 140, 20,
            hwnd, (HMENU)IDC_CENTER_DISPLAY, GetModuleHandle(NULL), NULL);
        // 设置当前热键显示
        GetModifierName(g_centerModifiers, _modBuf, 64);
        GetKeyName(g_centerKey, _keyBuf, 32);
        swprintf(keyText, 256, L"%s+%s", _modBuf, _keyBuf);
        SetWindowText(hCenterDisplay, keyText);

        hCenterButton = CreateWindow(L"BUTTON", L"设置",
            WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON,
            180, yPos, 50, 20,
            hwnd, (HMENU)IDC_CENTER_BUTTON, GetModuleHandle(NULL), NULL);

        yPos += 40;

        // 窗口抖动功能区
        CreateWindow(L"STATIC", L"窗口抖动:",
            WS_VISIBLE | WS_CHILD,
            20, yPos, 80, 20,
            hwnd, NULL, GetModuleHandle(NULL), NULL);

        hShakeEnable = CreateWindow(L"BUTTON", L"启用",
            WS_VISIBLE | WS_CHILD | BS_AUTOCHECKBOX,
            150, yPos, 60, 20,
            hwnd, (HMENU)IDC_SHAKE_ENABLE, GetModuleHandle(NULL), NULL);
        SendMessage(hShakeEnable, BM_SETCHECK, g_enableShake ? BST_CHECKED : BST_UNCHECKED, 0);

        yPos += 30;

        hShakeDisplay = CreateWindow(L"EDIT", L"",
            WS_VISIBLE | WS_CHILD | WS_BORDER | ES_AUTOHSCROLL | ES_READONLY,
            30, yPos, 140, 20,
            hwnd, (HMENU)IDC_SHAKE_DISPLAY, GetModuleHandle(NULL), NULL);
        // 设置当前热键显示
        GetModifierName(g_shakeModifiers, _modBuf, 64);
        GetKeyName(g_shakeKey, _keyBuf, 32);
        swprintf(keyText, 256, L"%s+%s", _modBuf, _keyBuf);
        SetWindowText(hShakeDisplay, keyText);

        hShakeButton = CreateWindow(L"BUTTON", L"设置",
            WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON,
            180, yPos, 50, 20,
            hwnd, (HMENU)IDC_SHAKE_BUTTON, GetModuleHandle(NULL), NULL);

        yPos += 40;

        // 恢复不透明度功能区
        CreateWindow(L"STATIC", L"恢复不透明:",
            WS_VISIBLE | WS_CHILD,
            20, yPos, 80, 20,
            hwnd, NULL, GetModuleHandle(NULL), NULL);

        hRestoreEnable = CreateWindow(L"BUTTON", L"启用",
            WS_VISIBLE | WS_CHILD | BS_AUTOCHECKBOX,
            150, yPos, 60, 20,
            hwnd, (HMENU)IDC_RESTORE_ENABLE, GetModuleHandle(NULL), NULL);
        SendMessage(hRestoreEnable, BM_SETCHECK, g_enableRestore ? BST_CHECKED : BST_UNCHECKED, 0);

        yPos += 30;

        hRestoreDisplay = CreateWindow(L"EDIT", L"",
            WS_VISIBLE | WS_CHILD | WS_BORDER | ES_AUTOHSCROLL | ES_READONLY,
            30, yPos, 140, 20,
            hwnd, (HMENU)IDC_RESTORE_DISPLAY, GetModuleHandle(NULL), NULL);
        // 设置当前热键显示
        GetModifierName(g_restoreModifiers, _modBuf, 64);
        GetKeyName(g_restoreKey, _keyBuf, 32);
        swprintf(keyText, 256, L"%s+%s", _modBuf, _keyBuf);
        SetWindowText(hRestoreDisplay, keyText);

        hRestoreButton = CreateWindow(L"BUTTON", L"设置",
            WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON,
            180, yPos, 50, 20,
            hwnd, (HMENU)IDC_RESTORE_BUTTON, GetModuleHandle(NULL), NULL);

        yPos += 40;

        // 透明度步长设置
        hTransparencyLabel = CreateWindow(L"STATIC", L"透明度步长: 10%",
            WS_VISIBLE | WS_CHILD,
            20, yPos, 200, 20,
            hwnd, (HMENU)IDC_TRANSPARENCY_LABEL, GetModuleHandle(NULL), NULL);

        yPos += 25;

        hTransparencySlider = CreateWindow(TRACKBAR_CLASS, L"",
            WS_VISIBLE | WS_CHILD | TBS_HORZ | TBS_AUTOTICKS,
            20, yPos, 250, 30,
            hwnd, (HMENU)IDC_TRANSPARENCY_SLIDER, GetModuleHandle(NULL), NULL);

        SendMessage(hTransparencySlider, TBM_SETRANGE, TRUE, MAKELONG(1, 50));
        SendMessage(hTransparencySlider, TBM_SETPOS, TRUE, g_transparencyStep);
        SendMessage(hTransparencySlider, TBM_SETTICFREQ, 5, 0);

        yPos += 50;

        // 保存按钮
        hSaveButton = CreateWindow(L"BUTTON", L"保存设置",
            WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON,
            20, yPos, 100, 30,
            hwnd, (HMENU)IDC_SAVE_BUTTON, GetModuleHandle(NULL), NULL);

        // 关闭按钮（放在保存按钮右侧）
        CreateWindow(L"BUTTON", L"关闭",
            WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON,
            140, yPos, 100, 30,
            hwnd, (HMENU)IDCANCEL, GetModuleHandle(NULL), NULL);

        break;
    }

    case WM_HSCROLL:
        if ((HWND)lParam == hTransparencySlider) {
            int pos = (int)SendMessage(hTransparencySlider, TBM_GETPOS, 0, 0);
            wchar_t text[50];
            swprintf(text, 50, L"透明度步长: %d%%", pos);
            SetWindowText(hTransparencyLabel, text);
        }
        break;

    case WM_COMMAND:
        switch (LOWORD(wParam)) {
        case IDC_TRANSPARENCY_UP_BUTTON:
        {
            if (g_isListeningHotkey && g_currentListeningType == 1) {
                g_isListeningHotkey = FALSE;
                g_currentListeningType = 0;
                SetWindowText(hTransparencyUpButton, L"设置");
                // 恢复原来的热键显示
                wchar_t keyText[256];
                wchar_t _modBuf[64], _keyBuf[32];
                GetModifierName(g_transparencyUpModifiers, _modBuf, 64);
                GetKeyName(g_transparencyUpKey, _keyBuf, 32);
                swprintf(keyText, 256, L"%s+%s", _modBuf, _keyBuf);
                SetWindowText(hTransparencyUpDisplay, keyText);
                ReleaseCapture();
            }
            else {
                g_isListeningHotkey = TRUE;
                g_currentListeningType = 1;
                g_hCurrentButton = hTransparencyUpButton;
                g_hCurrentDisplay = hTransparencyUpDisplay;
                g_listeningStartTime = GetTickCount();
                SetWindowText(hTransparencyUpButton, L"取消");
                SetWindowText(hTransparencyUpDisplay, L"请按下组合键...");
                SetFocus(hwnd);
                SetCapture(hwnd);
            }
            break;
        }
        case IDC_TRANSPARENCY_DOWN_BUTTON:
        {
            if (g_isListeningHotkey && g_currentListeningType == 2) {
                g_isListeningHotkey = FALSE;
                g_currentListeningType = 0;
                SetWindowText(hTransparencyDownButton, L"设置");
                // 恢复原来的热键显示
                wchar_t keyText[256];
                wchar_t _modBuf[64], _keyBuf[32];
                GetModifierName(g_transparencyDownModifiers, _modBuf, 64);
                GetKeyName(g_transparencyDownKey, _keyBuf, 32);
                swprintf(keyText, 256, L"%s+%s", _modBuf, _keyBuf);
                SetWindowText(hTransparencyDownDisplay, keyText);
                ReleaseCapture();
            }
            else {
                g_isListeningHotkey = TRUE;
                g_currentListeningType = 2;
                g_hCurrentButton = hTransparencyDownButton;
                g_hCurrentDisplay = hTransparencyDownDisplay;
                g_listeningStartTime = GetTickCount();
                SetWindowText(hTransparencyDownButton, L"取消");
                SetWindowText(hTransparencyDownDisplay, L"请按下组合键...");
                SetFocus(hwnd);
                SetCapture(hwnd);
            }
            break;
        }
        case IDC_CENTER_BUTTON:
        {
            if (g_isListeningHotkey && g_currentListeningType == 3) {
                g_isListeningHotkey = FALSE;
                g_currentListeningType = 0;
                SetWindowText(hCenterButton, L"设置");
                // 恢复原来的热键显示
                wchar_t keyText[256];
                wchar_t _modBuf[64], _keyBuf[32];
                GetModifierName(g_centerModifiers, _modBuf, 64);
                GetKeyName(g_centerKey, _keyBuf, 32);
                swprintf(keyText, 256, L"%s+%s", _modBuf, _keyBuf);
                SetWindowText(hCenterDisplay, keyText);
                ReleaseCapture();
            }
            else {
                g_isListeningHotkey = TRUE;
                g_currentListeningType = 3;
                g_hCurrentButton = hCenterButton;
                g_hCurrentDisplay = hCenterDisplay;
                g_listeningStartTime = GetTickCount();
                SetWindowText(hCenterButton, L"取消");
                SetWindowText(hCenterDisplay, L"请按下组合键...");
                SetFocus(hwnd);
                SetCapture(hwnd);
            }
            break;
        }
        case IDC_SHAKE_BUTTON:
        {
            if (g_isListeningHotkey && g_currentListeningType == 4) {
                g_isListeningHotkey = FALSE;
                g_currentListeningType = 0;
                SetWindowText(hShakeButton, L"设置");
                // 恢复原来的热键显示
                wchar_t keyText[256];
                wchar_t _modBuf[64], _keyBuf[32];
                GetModifierName(g_shakeModifiers, _modBuf, 64);
                GetKeyName(g_shakeKey, _keyBuf, 32);
                swprintf(keyText, 256, L"%s+%s", _modBuf, _keyBuf);
                SetWindowText(hShakeDisplay, keyText);
                ReleaseCapture();
            }
            else {
                g_isListeningHotkey = TRUE;
                g_currentListeningType = 4;
                g_hCurrentButton = hShakeButton;
                g_hCurrentDisplay = hShakeDisplay;
                g_listeningStartTime = GetTickCount();
                SetWindowText(hShakeButton, L"取消");
                SetWindowText(hShakeDisplay, L"请按下组合键...");
                SetFocus(hwnd);
                SetCapture(hwnd);
            }
            break;
        }
        case IDC_RESTORE_BUTTON:
        {
            if (g_isListeningHotkey && g_currentListeningType == 5) {
                g_isListeningHotkey = FALSE;
                g_currentListeningType = 0;
                SetWindowText(hRestoreButton, L"设置");
                wchar_t keyText[256];
                wchar_t _modBuf[64], _keyBuf[32];
                GetModifierName(g_restoreModifiers, _modBuf, 64);
                GetKeyName(g_restoreKey, _keyBuf, 32);
                swprintf(keyText, 256, L"%s+%s", _modBuf, _keyBuf);
                SetWindowText(hRestoreDisplay, keyText);
                ReleaseCapture();
            }
            else {
                g_isListeningHotkey = TRUE;
                g_currentListeningType = 5;
                g_hCurrentButton = hRestoreButton;
                g_hCurrentDisplay = hRestoreDisplay;
                g_listeningStartTime = GetTickCount();
                SetWindowText(hRestoreButton, L"取消");
                SetWindowText(hRestoreDisplay, L"请按下组合键...");
                SetFocus(hwnd);
                SetCapture(hwnd);
            }
            break;
        }
        case IDC_RESTORE_ENABLE:
            g_enableRestore = (SendMessage(hRestoreEnable, BM_GETCHECK, 0, 0) == BST_CHECKED);
            break;
        case IDC_TRANSPARENCY_ENABLE:
        {
            BOOL transparencyEnabled = (SendMessage(hTransparencyEnable, BM_GETCHECK, 0, 0) == BST_CHECKED);
            g_enableTransparencyUp = transparencyEnabled;
            g_enableTransparencyDown = transparencyEnabled;
            break;
        }
        case IDC_CENTER_ENABLE:
            g_enableCenter = (SendMessage(hCenterEnable, BM_GETCHECK, 0, 0) == BST_CHECKED);
            break;
        case IDC_SHAKE_ENABLE:
            g_enableShake = (SendMessage(hShakeEnable, BM_GETCHECK, 0, 0) == BST_CHECKED);
            break;
        case IDC_SAVE_BUTTON:
        {
            // 获取透明度步长
            g_transparencyStep = (int)SendMessage(hTransparencySlider, TBM_GETPOS, 0, 0);

            // 保存配置
            SaveConfig();

            // 重新注册热键
            UnregisterHotKeys(g_hMainWnd);
            RegisterHotKeys(g_hMainWnd);

            MessageBox(hwnd, L"设置已保存！", L"提示", MB_OK | MB_ICONINFORMATION);
            break;
        }
        case IDCANCEL:
            // 取消设置时，重新注册所有热键
            RegisterHotKeys(g_hMainWnd);
            DestroyWindow(hwnd);
            g_hSettingsWnd = NULL;
            break;
        }
        break;

    case WM_KEYDOWN:
    case WM_SYSKEYDOWN:
        if (g_isListeningHotkey) {
            // 检查超时（10秒）
            if (GetTickCount() - g_listeningStartTime > 10000) {
                g_isListeningHotkey = FALSE;
                SetWindowText(g_hCurrentButton, L"设置");
                SetWindowText(g_hCurrentDisplay, L"监听超时，请重试");
                g_currentListeningType = 0;
                ReleaseCapture();
                break;
            }

            // 检测修饰键
            UINT modifiers = 0;
            if (GetAsyncKeyState(VK_CONTROL) & 0x8000) modifiers |= MOD_CONTROL;
            if (GetAsyncKeyState(VK_MENU) & 0x8000) modifiers |= MOD_ALT;
            if (GetAsyncKeyState(VK_SHIFT) & 0x8000) modifiers |= MOD_SHIFT;
            if (GetAsyncKeyState(VK_LWIN) & 0x8000 || GetAsyncKeyState(VK_RWIN) & 0x8000) modifiers |= MOD_WIN;

            // 如果没有修饰键，忽略单独按键（除了功能键）
            if (modifiers == 0 && wParam >= 'A' && wParam <= 'Z') {
                break;
            }

            // 验证热键有效性：必须有修饰键或者是功能键
            if (modifiers == 0 && !(wParam >= VK_F1 && wParam <= VK_F24) &&
                wParam != VK_INSERT && wParam != VK_DELETE && wParam != VK_HOME &&
                wParam != VK_END && wParam != VK_PRIOR && wParam != VK_NEXT &&
                wParam != VK_SPACE && wParam != VK_TAB && wParam != VK_RETURN && wParam != VK_ESCAPE) {
                SetWindowText(g_hCurrentDisplay, L"请使用修饰键组合");
                break;
            }

            // 处理功能键、方向键、字母键和数字键
            if ((wParam >= VK_F1 && wParam <= VK_F24) ||
                (wParam >= VK_LEFT && wParam <= VK_DOWN) ||
                (wParam >= VK_NUMPAD0 && wParam <= VK_NUMPAD9) ||
                (wParam >= 'A' && wParam <= 'Z') ||
                (wParam >= '0' && wParam <= '9') ||
                wParam == VK_INSERT || wParam == VK_DELETE ||
                wParam == VK_HOME || wParam == VK_END ||
                wParam == VK_PRIOR || wParam == VK_NEXT ||
                wParam == VK_SPACE || wParam == VK_TAB ||
                wParam == VK_RETURN || wParam == VK_ESCAPE) {

                // 根据当前监听类型更新对应的全局变量
                switch (g_currentListeningType) {
                case 1: // 透明度增加
                    g_transparencyUpModifiers = modifiers;
                    g_transparencyUpKey = (UINT)wParam;
                    break;
                case 2: // 透明度减少
                    g_transparencyDownModifiers = modifiers;
                    g_transparencyDownKey = (UINT)wParam;
                    break;
                case 3: // 窗口居中
                    g_centerModifiers = modifiers;
                    g_centerKey = (UINT)wParam;
                    break;
                case 4: // 窗口抖动
                    g_shakeModifiers = modifiers;
                    g_shakeKey = (UINT)wParam;
                    break;
                case 5: // 恢复不透明度
                    g_restoreModifiers = modifiers;
                    g_restoreKey = (UINT)wParam;
                    break;
                }

                // 显示设置的热键
                wchar_t hotkeyText[100] = L"";
                if (modifiers & MOD_CONTROL) wcscat_s(hotkeyText, 100, L"CTRL+");
                if (modifiers & MOD_ALT) wcscat_s(hotkeyText, 100, L"ALT+");
                if (modifiers & MOD_SHIFT) wcscat_s(hotkeyText, 100, L"SHIFT+");
                if (modifiers & MOD_WIN) wcscat_s(hotkeyText, 100, L"WIN+");

                // 添加按键名称
                switch (wParam) {
                case VK_UP: wcscat_s(hotkeyText, 100, L"UP"); break;
                case VK_DOWN: wcscat_s(hotkeyText, 100, L"DOWN"); break;
                case VK_LEFT: wcscat_s(hotkeyText, 100, L"LEFT"); break;
                case VK_RIGHT: wcscat_s(hotkeyText, 100, L"RIGHT"); break;
                case VK_NUMPAD0: wcscat_s(hotkeyText, 100, L"NUM0"); break;
                case VK_NUMPAD1: wcscat_s(hotkeyText, 100, L"NUM1"); break;
                case VK_NUMPAD2: wcscat_s(hotkeyText, 100, L"NUM2"); break;
                case VK_NUMPAD3: wcscat_s(hotkeyText, 100, L"NUM3"); break;
                case VK_NUMPAD4: wcscat_s(hotkeyText, 100, L"NUM4"); break;
                case VK_NUMPAD5: wcscat_s(hotkeyText, 100, L"NUM5"); break;
                case VK_NUMPAD6: wcscat_s(hotkeyText, 100, L"NUM6"); break;
                case VK_NUMPAD7: wcscat_s(hotkeyText, 100, L"NUM7"); break;
                case VK_NUMPAD8: wcscat_s(hotkeyText, 100, L"NUM8"); break;
                case VK_NUMPAD9: wcscat_s(hotkeyText, 100, L"NUM9"); break;
                case VK_INSERT: wcscat_s(hotkeyText, 100, L"INSERT"); break;
                case VK_DELETE: wcscat_s(hotkeyText, 100, L"DELETE"); break;
                case VK_HOME: wcscat_s(hotkeyText, 100, L"HOME"); break;
                case VK_END: wcscat_s(hotkeyText, 100, L"END"); break;
                case VK_PRIOR: wcscat_s(hotkeyText, 100, L"PAGEUP"); break;
                case VK_NEXT: wcscat_s(hotkeyText, 100, L"PAGEDOWN"); break;
                case VK_SPACE: wcscat_s(hotkeyText, 100, L"SPACE"); break;
                case VK_TAB: wcscat_s(hotkeyText, 100, L"TAB"); break;
                case VK_RETURN: wcscat_s(hotkeyText, 100, L"ENTER"); break;
                case VK_ESCAPE: wcscat_s(hotkeyText, 100, L"ESC"); break;
                default:
                    if (wParam >= VK_F1 && wParam <= VK_F24) {
                        wchar_t fKey[8];
                        swprintf(fKey, 8, L"F%d", (int)(wParam - VK_F1 + 1));
                        wcscat_s(hotkeyText, 100, fKey);
                    }
                    else if (wParam >= 'A' && wParam <= 'Z') {
                        wchar_t letter[2] = { (wchar_t)wParam, L'\0' };
                        wcscat_s(hotkeyText, 100, letter);
                    }
                    else if (wParam >= '0' && wParam <= '9') {
                        wchar_t digit[2] = { (wchar_t)wParam, L'\0' };
                        wcscat_s(hotkeyText, 100, digit);
                    }
                    else {
                        wchar_t keyName[32];
                        swprintf(keyName, 32, L"KEY_%d", (int)wParam);
                        wcscat_s(hotkeyText, 100, keyName);
                    }
                    break;
                }

                SetWindowText(g_hCurrentDisplay, hotkeyText);

                // 停止监听
                g_isListeningHotkey = FALSE;
                g_currentListeningType = 0;
                SetWindowText(g_hCurrentButton, L"设置");
                ReleaseCapture();
            }
        }
        break;

    case WM_CLOSE:
        // 关闭设置窗口时，重新注册所有热键
        RegisterHotKeys(g_hMainWnd);
        DestroyWindow(hwnd);
        g_hSettingsWnd = NULL;
        break;

    default:
        return DefWindowProc(hwnd, uMsg, wParam, lParam);
    }
    return 0;
}

// 创建系统托盘图标
void CreateTrayIcon(HWND hwnd)
{
    ZeroMemory(&g_nid, sizeof(NOTIFYICONDATA));
    g_nid.cbSize = sizeof(NOTIFYICONDATA);
    g_nid.hWnd = hwnd;
    g_nid.uID = 1;
    g_nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    g_nid.uCallbackMessage = WM_TRAYICON;

    // 直接加载图标资源
    g_nid.hIcon = LoadIcon(GetModuleHandle(NULL), MAKEINTRESOURCE(IDI_WINDOW2CLEAR));
    if (!g_nid.hIcon) {
        // 如果无法加载自定义图标，使用默认应用程序图标
        g_nid.hIcon = LoadIcon(NULL, IDI_APPLICATION);
    }

    swprintf_s(g_nid.szTip, sizeof(g_nid.szTip) / sizeof(wchar_t), L"Window2Clear %s - 窗口透明度控制", APP_VERSION);

    Shell_NotifyIcon(NIM_ADD, &g_nid);
}

// 移除系统托盘图标
void RemoveTrayIcon()
{
    Shell_NotifyIcon(NIM_DELETE, &g_nid);
}

// 显示右键菜单
void ShowContextMenu(HWND hwnd)
{
    HMENU hMenu = CreatePopupMenu();
    AppendMenu(hMenu, MF_STRING, ID_TRAY_SETTINGS, L"设置");
    AppendMenu(hMenu, MF_SEPARATOR, 0, NULL);
    AppendMenu(hMenu, MF_STRING, ID_TRAY_EXIT, L"退出");

    POINT pt;
    GetCursorPos(&pt);

    SetForegroundWindow(hwnd);
    TrackPopupMenu(hMenu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, NULL);
    DestroyMenu(hMenu);
}

// 显示设置窗口
void ShowSettingsWindow()
{
    if (g_hSettingsWnd) {
        SetForegroundWindow(g_hSettingsWnd);
        return;
    }

    // 注册设置窗口类（仅首次）
    WNDCLASS wc = { 0 };
    if (!GetClassInfo(GetModuleHandle(NULL), L"SettingsWindowClass", &wc)) {
        ZeroMemory(&wc, sizeof(wc));
        wc.lpfnWndProc = SettingsProc;
        wc.hInstance = GetModuleHandle(NULL);
        wc.lpszClassName = L"SettingsWindowClass";
        wc.hCursor = LoadCursor(NULL, IDC_ARROW);
        wc.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);
        wc.hIcon = LoadIcon(GetModuleHandle(NULL), MAKEINTRESOURCE(IDI_WINDOW2CLEAR));
        RegisterClass(&wc);
    }

    // 计算窗口居中位置（基于当前鼠标所在显示器）
    int windowWidth = 380;
    int windowHeight = 520;
    POINT cursorPos;
    GetCursorPos(&cursorPos);
    HMONITOR hMon = MonitorFromPoint(cursorPos, MONITOR_DEFAULTTONEAREST);
    MONITORINFO mi = { sizeof(mi) };
    GetMonitorInfo(hMon, &mi);
    RECT workArea = mi.rcWork;
    int x = workArea.left + (workArea.right - workArea.left - windowWidth) / 2;
    int y = workArea.top + (workArea.bottom - workArea.top - windowHeight) / 2;

    // 创建设置窗口
    wchar_t windowTitle[100];
    swprintf_s(windowTitle, 100, L"Window2Clear %s 设置", APP_VERSION);
    g_hSettingsWnd = CreateWindow(
        L"SettingsWindowClass",
        windowTitle,
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        x, y,
        windowWidth, windowHeight,
        NULL, NULL, GetModuleHandle(NULL), NULL
    );

    if (g_hSettingsWnd) {
        ShowWindow(g_hSettingsWnd, SW_SHOW);
        UpdateWindow(g_hSettingsWnd);
    }
}

// 注册单个热键，失败时尝试用默认值回退
static void RegisterOneHotkey(HWND hwnd, int id, BOOL enable, UINT mods, UINT key, UINT defMods, UINT defKey, LPCWSTR name)
{
    UnregisterHotKey(hwnd, id);
    if (!enable) return;
    if (RegisterHotKey(hwnd, id, mods, key)) return;
    // 当前热键冲突，尝试用默认值注册
    if (mods == defMods && key == defKey) return; // 已经是默认值，没救了
    if (RegisterHotKey(hwnd, id, defMods, defKey)) {
        wchar_t msg[200];
        swprintf_s(msg, 200, L"%s热键注册失败（可能与其他程序冲突），已恢复为默认热键", name);
        MessageBox(hwnd, msg, L"警告", MB_OK | MB_ICONWARNING);
    } else {
        wchar_t msg[200];
        swprintf_s(msg, 200, L"%s热键注册失败，可能与其他程序冲突", name);
        MessageBox(hwnd, msg, L"警告", MB_OK | MB_ICONWARNING);
    }
}

// 注册全局热键
void RegisterHotKeys(HWND hwnd)
{
    RegisterOneHotkey(hwnd, ID_HOTKEY_TRANSPARENCY_UP, g_enableTransparencyUp,
        g_transparencyUpModifiers, g_transparencyUpKey, MOD_ALT, VK_LEFT, L"透明度增加");
    RegisterOneHotkey(hwnd, ID_HOTKEY_TRANSPARENCY_DOWN, g_enableTransparencyDown,
        g_transparencyDownModifiers, g_transparencyDownKey, MOD_ALT, VK_RIGHT, L"透明度减少");
    RegisterOneHotkey(hwnd, ID_HOTKEY_CENTER_WINDOW, g_enableCenter,
        g_centerModifiers, g_centerKey, MOD_CONTROL, VK_NUMPAD5, L"窗口居中");
    RegisterOneHotkey(hwnd, ID_HOTKEY_SHAKE_WINDOW, g_enableShake,
        g_shakeModifiers, g_shakeKey, MOD_ALT, VK_DOWN, L"窗口抖动");
    RegisterOneHotkey(hwnd, ID_HOTKEY_RESTORE_OPACITY, g_enableRestore,
        g_restoreModifiers, g_restoreKey, MOD_ALT, VK_UP, L"恢复透明度");
}

// 注销全局热键
void UnregisterHotKeys(HWND hwnd)
{
    UnregisterHotKey(hwnd, ID_HOTKEY_TRANSPARENCY_UP);
    UnregisterHotKey(hwnd, ID_HOTKEY_TRANSPARENCY_DOWN);
    UnregisterHotKey(hwnd, ID_HOTKEY_CENTER_WINDOW);
    UnregisterHotKey(hwnd, ID_HOTKEY_SHAKE_WINDOW);
    UnregisterHotKey(hwnd, ID_HOTKEY_RESTORE_OPACITY);
}

// 调整窗口透明度
void AdjustWindowTransparency(BOOL increase)
{
    HWND hwnd = GetTopMostWindow();
    if (!hwnd || !IsWindow(hwnd) || hwnd == g_hMainWnd || hwnd == g_hSettingsWnd) {
        return;
    }

    int currentAlpha = GetWindowTransparency(hwnd);
    int newAlpha;

    int delta = MulDiv(255, g_transparencyStep, 100);
    if (delta < 1) delta = 1;

    if (increase) {
        newAlpha = currentAlpha - delta;
        if (newAlpha < 25) newAlpha = 25;
    }
    else {
        newAlpha = currentAlpha + delta;
        if (newAlpha > 255) newAlpha = 255;
    }

    SetWindowTransparency(hwnd, newAlpha);
}

// 获取最上层窗口
HWND GetTopMostWindow()
{
    HWND hwnd = GetForegroundWindow();
    if (!hwnd || !IsWindow(hwnd) || IsIconic(hwnd)) {
        return NULL;
    }
    return hwnd;
}

// 设置窗口透明度
// 只用 SetWindowLong + SetLayeredWindowAttributes，不调 SetWindowPos
// 不加 WS_EX_TOPMOST，避免目标窗口因 z-order 变化重置自身样式
void SetWindowTransparency(HWND hwnd, int alpha)
{
    LONG exStyle = GetWindowLong(hwnd, GWL_EXSTYLE);
    if (alpha < 255) {
        SetWindowLong(hwnd, GWL_EXSTYLE, exStyle | WS_EX_LAYERED);
        SetLayeredWindowAttributes(hwnd, 0, (BYTE)alpha, LWA_ALPHA);
        TrackTransparentWindow(hwnd, alpha);
    }
    else {
        SetWindowLong(hwnd, GWL_EXSTYLE, exStyle & ~WS_EX_LAYERED);
        UntrackTransparentWindow(hwnd);
    }
}

// 获取窗口透明度
int GetWindowTransparency(HWND hwnd)
{
    LONG exStyle = GetWindowLong(hwnd, GWL_EXSTYLE);
    if (exStyle & WS_EX_LAYERED) {
        BYTE alpha;
        COLORREF colorKey;
        DWORD flags;
        if (GetLayeredWindowAttributes(hwnd, &colorKey, &alpha, &flags)) {
            return (int)alpha;
        }
    }
    return 255; // 默认完全不透明
}

// 窗口居中
void CenterWindow()
{
    HWND hwnd = GetTopMostWindow();
    if (!hwnd || !IsWindow(hwnd) || hwnd == g_hMainWnd || hwnd == g_hSettingsWnd) {
        return;
    }

    // 获取窗口所在显示器的工作区（扣掉任务栏）
    HMONITOR hMon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    MONITORINFO mi = { sizeof(mi) };
    GetMonitorInfo(hMon, &mi);
    RECT workArea = mi.rcWork;

    RECT rect;
    GetWindowRect(hwnd, &rect);
    int windowWidth = rect.right - rect.left;
    int windowHeight = rect.bottom - rect.top;

    int x = workArea.left + (workArea.right - workArea.left - windowWidth) / 2;
    int y = workArea.top + (workArea.bottom - workArea.top - windowHeight) / 2;

    SetWindowPos(hwnd, NULL, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
}

// 窗口抖动状态（在 ShakeWindow 中使用，定时器回调在 WindowProc 中）

void ShakeWindow()
{
    HWND hwnd = GetTopMostWindow();
    if (!hwnd || hwnd == g_hMainWnd || hwnd == g_hSettingsWnd || !IsWindow(hwnd)) {
        return;
    }
    // 如果正在抖动，忽略
    if (g_shakeHwnd) return;

    RECT rect;
    GetWindowRect(hwnd, &rect);
    g_shakeHwnd = hwnd;
    g_shakeOrigX = rect.left;
    g_shakeOrigY = rect.top;
    g_shakeStep = 0;
    SetTimer(g_hMainWnd, SHAKE_TIMER_ID, 50, NULL);
}

// 恢复窗口不透明
void RestoreWindowOpacity()
{
    HWND hwnd = GetTopMostWindow();
    if (!hwnd || !IsWindow(hwnd) || hwnd == g_hMainWnd || hwnd == g_hSettingsWnd) {
        return;
    }

    int currentAlpha = GetWindowTransparency(hwnd);
    if (currentAlpha < 255) {
        SetWindowTransparency(hwnd, 255);  // 会自动 UntrackTransparentWindow
    }
}

// 查找已跟踪的透明窗口，返回索引，未找到返回 -1
int FindTransparentWindow(HWND hwnd)
{
    for (int i = 0; i < g_transparentCount; i++) {
        if (g_transparentWindows[i].hwnd == hwnd) {
            return i;
        }
    }
    return -1;
}

// 跟踪透明窗口
void TrackTransparentWindow(HWND hwnd, int alpha)
{
    int idx = FindTransparentWindow(hwnd);
    if (idx >= 0) {
        g_transparentWindows[idx].alpha = alpha;
        return;
    }
    if (g_transparentCount >= MAX_TRANSPARENT_WINDOWS) {
        MessageBox(NULL, L"透明窗口数量已达上限（64个），无法继续跟踪新窗口。\n请先恢复部分窗口的透明度。",
            L"Window2Clear 提示", MB_OK | MB_ICONWARNING);
        return;
    }
    g_transparentWindows[g_transparentCount].hwnd = hwnd;
    g_transparentWindows[g_transparentCount].alpha = alpha;
    g_transparentCount++;
}

// 取消跟踪透明窗口
void UntrackTransparentWindow(HWND hwnd)
{
    int idx = FindTransparentWindow(hwnd);
    if (idx < 0) return;
    if (idx < g_transparentCount - 1) {
        g_transparentWindows[idx] = g_transparentWindows[g_transparentCount - 1];
    }
    g_transparentCount--;
}

// 校验热键修饰键（只能是 MOD_ALT/MOD_CONTROL/MOD_SHIFT/MOD_WIN 的组合）
static UINT ValidateModifiers(UINT mods, UINT def)
{
    UINT valid = 0;
    if (mods & MOD_ALT) valid |= MOD_ALT;
    if (mods & MOD_CONTROL) valid |= MOD_CONTROL;
    if (mods & MOD_SHIFT) valid |= MOD_SHIFT;
    if (mods & MOD_WIN) valid |= MOD_WIN;
    return valid ? valid : def;
}

// 校验虚拟键码（1-255 范围内）
static UINT ValidateVKey(UINT vk, UINT def)
{
    return (vk >= 1 && vk <= 255) ? vk : def;
}

// 加载配置
void LoadConfig()
{
    g_transparencyStep = GetPrivateProfileInt(L"Settings", L"TransparencyStep", 10, g_configFile);
    if (g_transparencyStep < 1) g_transparencyStep = 1;
    if (g_transparencyStep > 50) g_transparencyStep = 50;

    // 加载并校验热键配置
    g_transparencyUpModifiers = ValidateModifiers(GetPrivateProfileInt(L"Hotkeys", L"TransparencyUpModifiers", MOD_ALT, g_configFile), MOD_ALT);
    g_transparencyUpKey = ValidateVKey(GetPrivateProfileInt(L"Hotkeys", L"TransparencyUpKey", VK_LEFT, g_configFile), VK_LEFT);
    g_transparencyDownModifiers = ValidateModifiers(GetPrivateProfileInt(L"Hotkeys", L"TransparencyDownModifiers", MOD_ALT, g_configFile), MOD_ALT);
    g_transparencyDownKey = ValidateVKey(GetPrivateProfileInt(L"Hotkeys", L"TransparencyDownKey", VK_RIGHT, g_configFile), VK_RIGHT);
    g_centerModifiers = ValidateModifiers(GetPrivateProfileInt(L"Hotkeys", L"CenterModifiers", MOD_CONTROL, g_configFile), MOD_CONTROL);
    g_centerKey = ValidateVKey(GetPrivateProfileInt(L"Hotkeys", L"CenterKey", VK_NUMPAD5, g_configFile), VK_NUMPAD5);
    g_shakeModifiers = ValidateModifiers(GetPrivateProfileInt(L"Hotkeys", L"ShakeModifiers", MOD_ALT, g_configFile), MOD_ALT);
    g_shakeKey = ValidateVKey(GetPrivateProfileInt(L"Hotkeys", L"ShakeKey", VK_DOWN, g_configFile), VK_DOWN);
    g_restoreModifiers = ValidateModifiers(GetPrivateProfileInt(L"Hotkeys", L"RestoreModifiers", MOD_ALT, g_configFile), MOD_ALT);
    g_restoreKey = ValidateVKey(GetPrivateProfileInt(L"Hotkeys", L"RestoreKey", VK_UP, g_configFile), VK_UP);

    // 加载热键开关状态
    g_enableTransparencyUp = GetPrivateProfileInt(L"Switches", L"EnableTransparencyUp", 1, g_configFile);
    g_enableTransparencyDown = GetPrivateProfileInt(L"Switches", L"EnableTransparencyDown", 1, g_configFile);
    g_enableCenter = GetPrivateProfileInt(L"Switches", L"EnableCenter", 0, g_configFile);
    g_enableShake = GetPrivateProfileInt(L"Switches", L"EnableShake", 0, g_configFile);
    g_enableRestore = GetPrivateProfileInt(L"Switches", L"EnableRestore", 1, g_configFile);

    // 限制透明度步长范围
    if (g_transparencyStep < 1) g_transparencyStep = 1;
    if (g_transparencyStep > 50) g_transparencyStep = 50;
}

// 保存配置
void SaveConfig()
{
    wchar_t value[32];

    // 保存透明度步长
    swprintf(value, 32, L"%d", g_transparencyStep);
    WritePrivateProfileString(L"Settings", L"TransparencyStep", value, g_configFile);

    // 保存热键配置
    swprintf(value, 32, L"%d", g_transparencyUpModifiers);
    WritePrivateProfileString(L"Hotkeys", L"TransparencyUpModifiers", value, g_configFile);
    swprintf(value, 32, L"%d", g_transparencyUpKey);
    WritePrivateProfileString(L"Hotkeys", L"TransparencyUpKey", value, g_configFile);
    swprintf(value, 32, L"%d", g_transparencyDownModifiers);
    WritePrivateProfileString(L"Hotkeys", L"TransparencyDownModifiers", value, g_configFile);
    swprintf(value, 32, L"%d", g_transparencyDownKey);
    WritePrivateProfileString(L"Hotkeys", L"TransparencyDownKey", value, g_configFile);
    swprintf(value, 32, L"%d", g_centerModifiers);
    WritePrivateProfileString(L"Hotkeys", L"CenterModifiers", value, g_configFile);
    swprintf(value, 32, L"%d", g_centerKey);
    WritePrivateProfileString(L"Hotkeys", L"CenterKey", value, g_configFile);
    swprintf(value, 32, L"%d", g_shakeModifiers);
    WritePrivateProfileString(L"Hotkeys", L"ShakeModifiers", value, g_configFile);
    swprintf(value, 32, L"%d", g_shakeKey);
    WritePrivateProfileString(L"Hotkeys", L"ShakeKey", value, g_configFile);
    swprintf(value, 32, L"%d", g_restoreModifiers);
    WritePrivateProfileString(L"Hotkeys", L"RestoreModifiers", value, g_configFile);
    swprintf(value, 32, L"%d", g_restoreKey);
    WritePrivateProfileString(L"Hotkeys", L"RestoreKey", value, g_configFile);

    // 保存热键开关状态
    swprintf(value, 32, L"%d", g_enableTransparencyUp ? 1 : 0);
    WritePrivateProfileString(L"Switches", L"EnableTransparencyUp", value, g_configFile);
    swprintf(value, 32, L"%d", g_enableTransparencyDown ? 1 : 0);
    WritePrivateProfileString(L"Switches", L"EnableTransparencyDown", value, g_configFile);
    swprintf(value, 32, L"%d", g_enableCenter ? 1 : 0);
    WritePrivateProfileString(L"Switches", L"EnableCenter", value, g_configFile);
    swprintf(value, 32, L"%d", g_enableShake ? 1 : 0);
    WritePrivateProfileString(L"Switches", L"EnableShake", value, g_configFile);
    swprintf(value, 32, L"%d", g_enableRestore ? 1 : 0);
    WritePrivateProfileString(L"Switches", L"EnableRestore", value, g_configFile);
}

// 获取修饰键名称
void GetModifierName(UINT modifiers, wchar_t* buf, size_t bufSize)
{
    buf[0] = L'\0';

    if (modifiers & MOD_CONTROL) {
        wcscat_s(buf, bufSize, L"CTRL");
    }
    if (modifiers & MOD_ALT) {
        if (wcslen(buf) > 0) wcscat_s(buf, bufSize, L"+");
        wcscat_s(buf, bufSize, L"ALT");
    }
    if (modifiers & MOD_SHIFT) {
        if (wcslen(buf) > 0) wcscat_s(buf, bufSize, L"+");
        wcscat_s(buf, bufSize, L"SHIFT");
    }
    if (modifiers & MOD_WIN) {
        if (wcslen(buf) > 0) wcscat_s(buf, bufSize, L"+");
        wcscat_s(buf, bufSize, L"WIN");
    }

    return;
}

// 获取按键名称
void GetKeyName(UINT vkCode, wchar_t* buf, size_t bufSize){

    switch (vkCode) {
        // 功能键
    case VK_F1:
        wcscpy_s(buf, bufSize, L"F1");
        return;
    case VK_F2:
        wcscpy_s(buf, bufSize, L"F2");
        return;
    case VK_F3:
        wcscpy_s(buf, bufSize, L"F3");
        return;
    case VK_F4:
        wcscpy_s(buf, bufSize, L"F4");
        return;
    case VK_F5:
        wcscpy_s(buf, bufSize, L"F5");
        return;
    case VK_F6:
        wcscpy_s(buf, bufSize, L"F6");
        return;
    case VK_F7:
        wcscpy_s(buf, bufSize, L"F7");
        return;
    case VK_F8:
        wcscpy_s(buf, bufSize, L"F8");
        return;
    case VK_F9:
        wcscpy_s(buf, bufSize, L"F9");
        return;
    case VK_F10:
        wcscpy_s(buf, bufSize, L"F10");
        return;
    case VK_F11:
        wcscpy_s(buf, bufSize, L"F11");
        return;
    case VK_F12:
        wcscpy_s(buf, bufSize, L"F12");
        return;

        // 方向键
    case VK_LEFT:
        wcscpy_s(buf, bufSize, L"LEFT");
        return;
    case VK_RIGHT:
        wcscpy_s(buf, bufSize, L"RIGHT");
        return;
    case VK_UP:
        wcscpy_s(buf, bufSize, L"UP");
        return;
    case VK_DOWN:
        wcscpy_s(buf, bufSize, L"DOWN");
        return;

        // 数字键盘
    case VK_NUMPAD0:
        wcscpy_s(buf, bufSize, L"NUM0");
        return;
    case VK_NUMPAD1:
        wcscpy_s(buf, bufSize, L"NUM1");
        return;
    case VK_NUMPAD2:
        wcscpy_s(buf, bufSize, L"NUM2");
        return;
    case VK_NUMPAD3:
        wcscpy_s(buf, bufSize, L"NUM3");
        return;
    case VK_NUMPAD4:
        wcscpy_s(buf, bufSize, L"NUM4");
        return;
    case VK_NUMPAD5:
        wcscpy_s(buf, bufSize, L"NUM5");
        return;
    case VK_NUMPAD6:
        wcscpy_s(buf, bufSize, L"NUM6");
        return;
    case VK_NUMPAD7:
        wcscpy_s(buf, bufSize, L"NUM7");
        return;
    case VK_NUMPAD8:
        wcscpy_s(buf, bufSize, L"NUM8");
        return;
    case VK_NUMPAD9:
        wcscpy_s(buf, bufSize, L"NUM9");
        return;

        // 特殊键
    case VK_INSERT:
        wcscpy_s(buf, bufSize, L"INSERT");
        return;
    case VK_DELETE:
        wcscpy_s(buf, bufSize, L"DELETE");
        return;
    case VK_HOME:
        wcscpy_s(buf, bufSize, L"HOME");
        return;
    case VK_END:
        wcscpy_s(buf, bufSize, L"END");
        return;
    case VK_PRIOR:
        wcscpy_s(buf, bufSize, L"PAGEUP");
        return;
    case VK_NEXT:
        wcscpy_s(buf, bufSize, L"PAGEDOWN");
        return;
    case VK_SPACE:
        wcscpy_s(buf, bufSize, L"SPACE");
        return;
    case VK_TAB:
        wcscpy_s(buf, bufSize, L"TAB");
        return;
    case VK_RETURN:
        wcscpy_s(buf, bufSize, L"ENTER");
        return;
    case VK_ESCAPE:
        wcscpy_s(buf, bufSize, L"ESC");
        return;

        // 字母和数字键
    default:
        if (vkCode >= 'A' && vkCode <= 'Z') {
            swprintf_s(buf, bufSize, L"%c", (wchar_t)vkCode);
            return;
        }
        if (vkCode >= '0' && vkCode <= '9') {
            swprintf_s(buf, bufSize, L"%c", (wchar_t)vkCode);
            return;
        }
        swprintf_s(buf, bufSize, L"KEY%d", vkCode);
        return;
    }
}