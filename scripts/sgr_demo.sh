#!/bin/bash
# VTE/SGR Demo Script for QuantaTerm
# Tests various SGR (Select Graphic Rendition) escape sequences

echo "=== QuantaTerm VTE/SGR Demonstration ==="
echo ""

echo "Basic text attributes:"
echo -e "Normal text"
echo -e "\e[1mBold text\e[0m"
echo -e "\e[3mItalic text\e[0m"
echo -e "\e[4mUnderlined text\e[0m"
echo -e "\e[7mReversed text\e[0m"
echo -e "\e[9mStrikethrough text\e[0m"
echo ""

echo "Attribute combinations:"
echo -e "\e[1;4mBold and underlined\e[0m"
echo -e "\e[1;3;4mBold, italic, and underlined\e[0m"
echo ""

echo "Standard colors:"
echo -e "\e[31mRed\e[0m \e[32mGreen\e[0m \e[33mYellow\e[0m \e[34mBlue\e[0m \e[35mMagenta\e[0m \e[36mCyan\e[0m"
echo ""

echo "Bright colors:"
echo -e "\e[91mBright Red\e[0m \e[92mBright Green\e[0m \e[93mBright Yellow\e[0m \e[94mBright Blue\e[0m \e[95mBright Magenta\e[0m \e[96mBright Cyan\e[0m"
echo ""

echo "256-color examples:"
echo -e "\e[38;5;196mBright Red (196)\e[0m"
echo -e "\e[38;5;46mBright Green (46)\e[0m"
echo -e "\e[38;5;21mBright Blue (21)\e[0m"
echo ""

echo "RGB/Truecolor examples:"
echo -e "\e[38;2;255;165;0mOrange (255,165,0)\e[0m"
echo -e "\e[38;2;75;0;130mIndigo (75,0,130)\e[0m"
echo -e "\e[38;2;238;130;238mViolet (238,130,238)\e[0m"
echo ""

echo "Combined formatting and colors:"
echo -e "\e[1;31mBold Red\e[0m"
echo -e "\e[3;34mItalic Blue\e[0m"
echo -e "\e[4;32mUnderlined Green\e[0m"
echo -e "\e[1;4;35mBold Underlined Magenta\e[0m"
echo ""

echo "=== Demo Complete ==="