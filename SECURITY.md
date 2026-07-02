# Security Policy

Report security issues privately to the maintainer before public disclosure.

TokenStack must never expose auth secrets, raw auth files, or mutation-capable connector behavior. Any finding involving `/consume`, auth leakage, unsafe endpoint registration, or persisted secrets is treated as high priority.

Do not include real tokens, full auth files, or private account responses in reports. Provide minimized redacted shapes instead.
