import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface GameSearchResult {
  name: string;
  steam_id?: string;
  developers?: string;
  publishers?: string;
}

interface GameSearchProps {
  onSelect: (game: GameSearchResult) => void;
}

export function GameSearch({ onSelect }: GameSearchProps) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<GameSearchResult[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSearch = async () => {
    if (!query.trim()) return;
    
    setIsLoading(true);
    setError(null);
    
    try {
      const data = await invoke<GameSearchResult[]>('search_pcgw_games', { query });
      setResults(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to search games');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="game-search">
      <div className="search-input-group" style={{ display: 'flex', gap: '8px', marginBottom: '16px' }}>
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
          placeholder="Search for a game (e.g. Witcher 3)..."
          style={{ flex: 1, padding: '8px', borderRadius: '4px', border: '1px solid #ccc' }}
        />
        <button 
          onClick={handleSearch}
          disabled={isLoading}
          style={{
            padding: '8px 16px',
            background: '#6366f1',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer'
          }}
        >
          {isLoading ? 'Searching...' : 'Search'}
        </button>
      </div>

      {error && <div className="error-message" style={{ color: 'red', marginBottom: '8px' }}>{error}</div>}

      <div className="search-results" style={{ maxHeight: '300px', overflowY: 'auto', border: '1px solid #eee', borderRadius: '4px' }}>
        {results.map((game, index) => (
          <div 
            key={index}
            onClick={() => onSelect(game)}
            style={{
              padding: '12px',
              borderBottom: '1px solid #eee',
              cursor: 'pointer',
              transition: 'background 0.2s'
            }}
            onMouseEnter={(e) => e.currentTarget.style.background = '#f5f5f5'}
            onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
          >
            <div style={{ fontWeight: 'bold' }}>{game.name}</div>
            <div style={{ fontSize: '0.8em', color: '#666' }}>
              {game.developers && <span>Dev: {game.developers}</span>}
              {game.publishers && <span> • Pub: {game.publishers}</span>}
            </div>
          </div>
        ))}
        {results.length === 0 && !isLoading && query && (
          <div style={{ padding: '12px', color: '#888', textAlign: 'center' }}>No results found</div>
        )}
      </div>
    </div>
  );
}
