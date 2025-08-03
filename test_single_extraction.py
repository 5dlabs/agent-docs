#!/usr/bin/env python3
"""Test BirdEye data extraction for a single page"""

import requests
import html
import urllib.parse
import json

def test_single_page():
    url = 'https://docs.birdeye.so/reference/get-defi-price'
    print(f"🧪 Testing extraction from: {url}")
    
    headers = {
        'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
        'Accept-Language': 'en-US,en;q=0.5',
        'Accept-Encoding': 'gzip, deflate, br',
        'Connection': 'keep-alive',
        'Upgrade-Insecure-Requests': '1',
    }
    
    try:
        response = requests.get(url, headers=headers, timeout=30)
        response.raise_for_status()
        content = response.text
        
        print(f"✅ Page loaded: {response.status_code}")
        print(f"📊 Content length: {len(content)}")
        
        # Look for data-initial-props (the pattern is: data-initial-props="{...)
        start_marker = 'data-initial-props="'
        start_pos = content.find(start_marker)
        
        if start_pos == -1:
            print("❌ data-initial-props not found")
            return None
            
        print(f"✅ Found data-initial-props at position: {start_pos}")
        
        # Extract the JSON data - find the content between the quotes
        start_pos += len(start_marker)
        end_pos = content.find('">', start_pos)  # Look for closing quote + >
        
        if end_pos == -1:
            # Try alternative ending patterns
            end_pos = content.find('" ', start_pos)  # quote + space
            if end_pos == -1:
                end_pos = content.find('"', start_pos)  # just quote
        
        if end_pos == -1:
            print("❌ Could not find end of data-initial-props")
            return None
            
        encoded_data = content[start_pos:end_pos]
        print(f"📏 Encoded data length: {len(encoded_data)}")
        
        # Decode HTML entities and URL encoding
        html_decoded = html.unescape(encoded_data)
        url_decoded = urllib.parse.unquote(html_decoded)
        
        # Parse as JSON
        try:
            data = json.loads(url_decoded)
            print(f"✅ Successfully parsed JSON!")
            print(f"📋 Top-level keys: {list(data.keys())}")
            
            # Look for API-related data
            if 'openapi' in data:
                print("✅ Found OpenAPI spec!")
                openapi_data = data['openapi']
                if 'paths' in openapi_data:
                    paths = openapi_data['paths']
                    print(f"📄 Found {len(paths)} API paths")
                    for path in list(paths.keys())[:3]:  # Show first 3
                        print(f"  - {path}")
                        
            return data
            
        except json.JSONDecodeError as e:
            print(f"❌ JSON decode error: {e}")
            # Show first 200 chars of decoded data for debugging
            print(f"📝 First 200 chars: {url_decoded[:200]}")
            return None
            
    except Exception as e:
        print(f"❌ Error: {e}")
        return None

if __name__ == "__main__":
    result = test_single_page()
    if result:
        print("🎉 Test successful! Data extraction works.")
    else:
        print("💥 Test failed - need to debug extraction logic.")