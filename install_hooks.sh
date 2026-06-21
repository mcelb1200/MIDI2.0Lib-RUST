#!/bin/bash
mkdir -p .git/hooks

echo '#!/bin/bash' > .git/hooks/pre-push
echo 'echo "🛡️ Running Sentinel Guardian security checks..."' >> .git/hooks/pre-push
echo 'python3 guardian.py' >> .git/hooks/pre-push
echo 'if [ $? -ne 0 ]; then echo "❌ Security regressions found. Push aborted."; exit 1; fi' >> .git/hooks/pre-push
echo 'echo "✅ Security checks passed."' >> .git/hooks/pre-push

chmod +x .git/hooks/pre-push
chmod +x install_hooks.sh
chmod +x guardian.py

echo "🛡️ Sentinel Guardian pre-push hook installed successfully."
