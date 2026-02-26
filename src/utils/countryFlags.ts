
export function codeToFlag(code: string): string {
  const upper = code.toUpperCase();
  if (upper.length !== 2) return "";
  const offset = 0x1f1e6 - 65; 
  return String.fromCodePoint(
    upper.charCodeAt(0) + offset,
    upper.charCodeAt(1) + offset,
  );
}

const KNOWN_CODES = new Set([
  "US", "GB", "UK", "DE", "FR", "RU", "NL", "CA", "AU", "JP",
  "KR", "SG", "IN", "BR", "IT", "ES", "SE", "NO", "FI", "DK",
  "PL", "CZ", "CH", "AT", "BE", "PT", "IE", "TR", "UA", "RO",
  "HU", "BG", "HR", "GR", "IL", "ZA", "AR", "MX", "CL", "CO",
  "HK", "TW", "TH", "VN", "PH", "ID", "MY", "NZ", "AE", "SA",
  "EG", "KE", "NG", "LT", "LV", "EE", "RS", "SK", "SI", "LU",
]);

function normalise(code: string): string {
  return code === "UK" ? "GB" : code;
}

export function extractCountryCode(name: string): string | null {
  
  const bracketMatch = name.match(/[\[(]([A-Za-z]{2})[\])]/);
  if (bracketMatch) {
    const code = bracketMatch[1].toUpperCase();
    if (KNOWN_CODES.has(code)) return normalise(code);
  }

  const tokens = name.toUpperCase().split(/[\s\-_.,;:]+/);
  for (const token of tokens) {
    if (token.length === 2 && KNOWN_CODES.has(token)) {
      return normalise(token);
    }
  }

  const lower = name.toLowerCase();
  const hints: Record<string, string> = {
    "united states": "US", "america": "US", "new york": "US", "los angeles": "US",
    "chicago": "US", "dallas": "US", "miami": "US", "seattle": "US",
    "germany": "DE", "frankfurt": "DE", "berlin": "DE", "munich": "DE",
    "france": "FR", "paris": "FR", "marseille": "FR",
    "russia": "RU", "moscow": "RU", "petersburg": "RU",
    "netherlands": "NL", "amsterdam": "NL", "holland": "NL",
    "canada": "CA", "toronto": "CA", "montreal": "CA", "vancouver": "CA",
    "australia": "AU", "sydney": "AU", "melbourne": "AU",
    "japan": "JP", "tokyo": "JP", "osaka": "JP",
    "korea": "KR", "seoul": "KR",
    "singapore": "SG",
    "india": "IN", "mumbai": "IN", "delhi": "IN",
    "brazil": "BR", "sao paulo": "BR",
    "italy": "IT", "milan": "IT", "rome": "IT",
    "spain": "ES", "madrid": "ES", "barcelona": "ES",
    "sweden": "SE", "stockholm": "SE",
    "norway": "NO", "oslo": "NO",
    "finland": "FI", "helsinki": "FI",
    "denmark": "DK", "copenhagen": "DK",
    "poland": "PL", "warsaw": "PL",
    "switzerland": "CH", "zurich": "CH",
    "austria": "AT", "vienna": "AT",
    "belgium": "BE", "brussels": "BE",
    "portugal": "PT", "lisbon": "PT",
    "ireland": "IE", "dublin": "IE",
    "turkey": "TR", "istanbul": "TR",
    "ukraine": "UA", "kyiv": "UA",
    "hong kong": "HK",
    "taiwan": "TW",
    "london": "GB", "manchester": "GB", "britain": "GB", "england": "GB",
  };

  for (const [hint, code] of Object.entries(hints)) {
    if (lower.includes(hint)) return code;
  }

  return null;
}

export function getFlagForName(name: string): string | null {
  const code = extractCountryCode(name);
  return code ? codeToFlag(code) : null;
}

export const detectCountryFlag = getFlagForName;

export function getInitials(name: string): string {
  const words = name.trim().split(/\s+/);
  if (words.length >= 2) {
    return (words[0][0] + words[1][0]).toUpperCase();
  }
  return name.slice(0, 2).toUpperCase();
}

export const COUNTRY_OPTIONS = [
  ...Array.from(KNOWN_CODES).sort().map((code) => ({
    value: code,
    label: `${codeToFlag(normalise(code))} ${code}`,
    flag: codeToFlag(normalise(code)),
  })),
];
