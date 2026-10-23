; SPDX-License-Identifier: GPL-3.0-or-later
; Hooks for the Windows installer built by the Tauri bundler (NSIS).
; See docs/releasing.md.

!macro NSIS_HOOK_PREINSTALL
  ; Stop the background program so an update can replace its files.
  nsExec::Exec 'taskkill /IM typlet.exe /F'
!macroend

!macro NSIS_HOOK_POSTINSTALL
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'taskkill /IM typlet.exe /F'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Remove the start-at-login entry the background program created.
  ; The user's snippets in %APPDATA%\Typlet are kept.
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Typlet"
!macroend
