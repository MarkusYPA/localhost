/* global React, ReactDOM, TweaksPanel, useTweaks, TweakSection, TweakRadio, TweakToggle, TweakSlider */
const { useEffect } = React;

const DEFAULT_TWEAKS = /*EDITMODE-BEGIN*/{
  "theme": "light",
  "accent": "#1E5BFF",
  "showGrid": true,
  "density": "comfortable"
}/*EDITMODE-END*/;

const ACCENTS = {
  blue:   '#1E5BFF',
  cyan:   '#0891B2',
  violet: '#5B21B6',
  amber:  '#B45309',
  green:  '#047857'
};

function applyTweaks(t) {
  const root = document.documentElement;
  root.style.setProperty('--accent', t.accent);
  // soft variant
  root.style.setProperty('--accent-soft', t.accent + '22');

  document.body.classList.toggle('theme-dark', t.theme === 'dark');
  document.body.classList.toggle('grid-off', !t.showGrid);
  document.body.classList.toggle('density-compact', t.density === 'compact');
}

function App() {
  const [tweaks, setTweak] = useTweaks(DEFAULT_TWEAKS);
  useEffect(() => { applyTweaks(tweaks); }, [tweaks]);

  return (
    <TweaksPanel title="Tweaks">
      <TweakSection title="Theme">
        <TweakRadio
          label="Mode"
          value={tweaks.theme}
          onChange={v => setTweak('theme', v)}
          options={[
            { value: 'light', label: 'Light' },
            { value: 'dark',  label: 'Dark'  }
          ]}
        />
        <TweakRadio
          label="Accent"
          value={Object.keys(ACCENTS).find(k => ACCENTS[k] === tweaks.accent) || 'blue'}
          onChange={v => setTweak('accent', ACCENTS[v])}
          options={[
            { value: 'blue',   label: 'Blue'   },
            { value: 'cyan',   label: 'Cyan'   },
            { value: 'violet', label: 'Violet' },
            { value: 'amber',  label: 'Amber'  },
            { value: 'green',  label: 'Green'  }
          ]}
        />
      </TweakSection>
      <TweakSection title="Layout">
        <TweakRadio
          label="Density"
          value={tweaks.density}
          onChange={v => setTweak('density', v)}
          options={[
            { value: 'comfortable', label: 'Comfortable' },
            { value: 'compact',     label: 'Compact'     }
          ]}
        />
        <TweakToggle
          label="Schematic grid"
          checked={tweaks.showGrid}
          onChange={v => setTweak('showGrid', v)}
        />
      </TweakSection>
    </TweaksPanel>
  );
}

ReactDOM.createRoot(document.getElementById('tweaks-root')).render(<App />);
