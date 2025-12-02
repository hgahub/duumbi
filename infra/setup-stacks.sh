#!/bin/bash
set -e

echo "🚀 Setting up Duumbi Infrastructure Stacks"
echo "==========================================="
echo ""

# Check if pulumi is installed
if ! command -v pulumi &> /dev/null; then
    echo "❌ Pulumi is not installed. Please install it first."
    exit 1
fi

echo "📦 Installing dependencies..."
npm install

echo ""
echo "🔧 Creating persistent stack..."
pulumi stack select persistent --create 2>/dev/null || pulumi stack select persistent
echo "✅ Persistent stack selected"

echo ""
echo "🚀 Deploying persistent infrastructure..."
pulumi up --yes

echo ""
echo "🔧 Creating temporary stack..."
pulumi stack select temporary --create 2>/dev/null || pulumi stack select temporary
echo "✅ Temporary stack selected"

echo ""
echo "🚀 Deploying temporary infrastructure..."
pulumi up --yes

echo ""
echo "✨ Setup complete!"
echo ""
echo "📝 Next steps:"
echo "1. Verify resources in Azure Portal"
echo "2. Read STACKS.md for usage instructions"
echo ""
echo "💡 To destroy temporary resources and save costs:"
echo "   pulumi stack select temporary && pulumi destroy"
echo ""
