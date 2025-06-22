#!/bin/bash

# Icon generation script for Clip Vault
# This script generates all required icon formats from the source icon.png
# Requires ImageMagick (magick command) and iconutil (macOS only for ICNS)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "icon.png" ]; then
    echo -e "${RED}Error: icon.png not found in current directory${NC}"
    echo "Please run this script from the icons directory containing icon.png"
    exit 1
fi

# Check if ImageMagick is installed
if ! command -v magick &> /dev/null; then
    echo -e "${RED}Error: ImageMagick not found${NC}"
    echo "Please install ImageMagick:"
    echo "  macOS: brew install imagemagick"
    echo "  Ubuntu: sudo apt-get install imagemagick"
    echo "  Windows: Download from https://imagemagick.org/script/download.php"
    exit 1
fi

echo -e "${BLUE}🎨 Generating icons from icon.png...${NC}"

# Function to create RGBA PNG icons
create_png_icon() {
    local size=$1
    local filename=$2
    echo -e "${YELLOW}Creating ${filename}...${NC}"
    magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize ${size} "${filename}"
}

# Create required PNG icons
echo -e "${BLUE}📱 Creating PNG icons...${NC}"
create_png_icon "32x32" "32x32.png"
create_png_icon "128x128" "128x128.png"
create_png_icon "256x256" "128x128@2x.png"

# Create additional common sizes
echo -e "${BLUE}📐 Creating additional PNG sizes...${NC}"
create_png_icon "16x16" "16x16.png"
create_png_icon "48x48" "48x48.png"
create_png_icon "64x64" "64x64.png"
create_png_icon "256x256" "256x256.png"
create_png_icon "512x512" "512x512.png"

# Create Windows Store app icons
echo -e "${BLUE}🪟 Creating Windows Store icons...${NC}"
create_png_icon "30x30" "Square30x30Logo.png"
create_png_icon "44x44" "Square44x44Logo.png"
create_png_icon "71x71" "Square71x71Logo.png"
create_png_icon "89x89" "Square89x89Logo.png"
create_png_icon "107x107" "Square107x107Logo.png"
create_png_icon "142x142" "Square142x142Logo.png"
create_png_icon "150x150" "Square150x150Logo.png"
create_png_icon "284x284" "Square284x284Logo.png"
create_png_icon "310x310" "Square310x310Logo.png"
create_png_icon "50x50" "StoreLogo.png"

# Create Windows ICO file
echo -e "${BLUE}🖼️ Creating Windows ICO file...${NC}"
echo -e "${YELLOW}Creating temporary files for ICO...${NC}"

# Create temporary RGBA files for ICO creation
magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 16x16 temp16.png
magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 32x32 temp32.png
magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 48x48 temp48.png
magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 64x64 temp64.png
magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 128x128 temp128.png
magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 256x256 temp256.png

echo -e "${YELLOW}Combining into icon.ico...${NC}"
magick temp16.png temp32.png temp48.png temp64.png temp128.png temp256.png icon.ico

# Clean up temporary files
rm temp*.png
echo -e "${GREEN}✅ ICO file created${NC}"

# Create macOS ICNS file
echo -e "${BLUE}🍎 Creating macOS ICNS file...${NC}"
if command -v iconutil &> /dev/null; then
    echo -e "${YELLOW}Creating iconset directory...${NC}"
    mkdir -p icon.iconset
    
    # Create all required sizes for macOS iconset
    magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 16x16 icon.iconset/icon_16x16.png
    magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 32x32 icon.iconset/icon_16x16@2x.png
    magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 32x32 icon.iconset/icon_32x32.png
    magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 64x64 icon.iconset/icon_32x32@2x.png
    magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 128x128 icon.iconset/icon_128x128.png
    magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 256x256 icon.iconset/icon_128x128@2x.png
    magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 256x256 icon.iconset/icon_256x256.png
    magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 512x512 icon.iconset/icon_256x256@2x.png
    magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 512x512 icon.iconset/icon_512x512.png
    magick icon.png -colorspace RGB -type TrueColorMatte -define png:color-type=6 -resize 1024x1024 icon.iconset/icon_512x512@2x.png
    
    echo -e "${YELLOW}Converting iconset to ICNS...${NC}"
    iconutil -c icns icon.iconset
    
    # Clean up iconset directory
    rm -rf icon.iconset
    echo -e "${GREEN}✅ ICNS file created${NC}"
else
    echo -e "${YELLOW}⚠️ iconutil not found (macOS only). Skipping ICNS creation.${NC}"
    echo "To create ICNS on macOS, iconutil is required (comes with Xcode Command Line Tools)"
fi

# Verify all files were created
echo -e "${BLUE}🔍 Verifying generated files...${NC}"

required_files=("32x32.png" "128x128.png" "128x128@2x.png" "icon.ico")
if command -v iconutil &> /dev/null; then
    required_files+=("icon.icns")
fi

missing_files=()
for file in "${required_files[@]}"; do
    if [ -f "$file" ]; then
        echo -e "${GREEN}✅ $file${NC}"
    else
        echo -e "${RED}❌ $file${NC}"
        missing_files+=("$file")
    fi
done

if [ ${#missing_files[@]} -eq 0 ]; then
    echo -e "${GREEN}🎉 All required icons generated successfully!${NC}"
    
    # Show file sizes
    echo -e "\n${BLUE}📊 File sizes:${NC}"
    ls -lh *.png *.ico *.icns 2>/dev/null | awk '{print $5 "\t" $9}' | sort -k2
    
    # Show formats
    echo -e "\n${BLUE}🔍 File formats:${NC}"
    for file in "${required_files[@]}"; do
        if [ -f "$file" ]; then
            echo -e "${YELLOW}$file:${NC} $(file "$file" | cut -d: -f2-)"
        fi
    done
    
else
    echo -e "${RED}❌ Some files failed to generate: ${missing_files[*]}${NC}"
    exit 1
fi

echo -e "\n${GREEN}✨ Icon generation complete!${NC}"
echo -e "${BLUE}📝 Usage notes:${NC}"
echo "  • All PNG files are in RGBA format as required by Tauri"
echo "  • ICO file contains multiple sizes for Windows"
echo "  • ICNS file is optimized for macOS (if available)"
echo "  • Additional sizes are provided for various platforms"