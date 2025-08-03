#!/usr/bin/env python3
"""Debug BirdEye endpoint discovery"""

import requests
from bs4 import BeautifulSoup

class TestDiscovery:
    def __init__(self):
        self.session = requests.Session()
        self.session.headers.update({
            'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
            'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
            'Accept-Language': 'en-US,en;q=0.5',
            'Accept-Encoding': 'gzip, deflate, br',
            'Connection': 'keep-alive',
            'Upgrade-Insecure-Requests': '1',
        })
        self.base_url = "https://docs.birdeye.so"

    def test_discovery(self):
        start_url = f"{self.base_url}/reference/get-defi-price"
        print(f"🔍 Testing discovery with: {start_url}")
        
        try:
            response = self.session.get(start_url, timeout=30)
            response.raise_for_status()
            print(f"✅ Status: {response.status_code}")
            
            soup = BeautifulSoup(response.text, 'html.parser')
            
            endpoint_urls = set()
            total_links = 0
            
            for link in soup.find_all('a', href=True):
                total_links += 1
                href = link['href']
                print(f"  Link {total_links}: {href}")
                
                if href.startswith('/reference/'):
                    full_url = f"{self.base_url}{href}"
                    endpoint_urls.add(full_url)
                    print(f"    ✅ Added: {full_url}")
                
                if total_links >= 10:  # Show first 10 for debugging
                    break
            
            print(f"\n📊 Total links found: {len(soup.find_all('a', href=True))}")
            print(f"📋 Reference URLs found: {len(endpoint_urls)}")
            
            for url in sorted(list(endpoint_urls))[:5]:
                print(f"  - {url}")
                
        except Exception as e:
            print(f"❌ Error: {e}")

if __name__ == "__main__":
    tester = TestDiscovery()
    tester.test_discovery()