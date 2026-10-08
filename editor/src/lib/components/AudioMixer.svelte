<script lang="ts">
  // Mixer de áudio (AUD-003): um canal por barramento do projeto, com volume, mudo, solo e os quatro efeitos.
  // Cada mudança é gravada no projeto e chega ao motor na hora, mesmo sem reprodução.
  import { projectSettings } from '$lib/state/settings.svelte';
  import { audio, VOLUME_MAX_DB, VOLUME_MIN_DB } from '$lib/state/audio.svelte';
  import NumberField from './NumberField.svelte';
  import { Headphones, Volume2, VolumeX } from '@lucide/svelte';
</script>

<div class="mixer">
  {#if projectSettings.buses.length === 0}
    <p class="empty">Este projeto não tem barramentos. Crie-os nas configurações do projeto.</p>
  {:else}
    {#each projectSettings.buses as bus (bus.name)}
      {@const solo = audio.isSolo(bus.name)}
      <section class="strip" class:solo class:muted={bus.mute} aria-label="Canal {bus.name}">
        <header>
          <span class="name" title={bus.name}>{bus.name}</span>
          <button
            type="button"
            class="tgl"
            class:on={bus.mute}
            aria-pressed={bus.mute}
            title={bus.mute ? 'Desmutar o canal' : 'Mutar o canal'}
            aria-label="Mudo: {bus.name}"
            onclick={() => audio.toggleMute(bus.name)}
          >
            {#if bus.mute}<VolumeX size={13} />{:else}<Volume2 size={13} />{/if}
          </button>
          <button
            type="button"
            class="tgl"
            class:on={solo}
            aria-pressed={solo}
            title="Solo: ouvir só este canal"
            aria-label="Solo: {bus.name}"
            onclick={() => audio.toggleSolo(bus.name)}
          >
            <Headphones size={13} />
          </button>
        </header>

        <div class="fader">
          <input
            type="range"
            min={VOLUME_MIN_DB}
            max={VOLUME_MAX_DB}
            step="0.5"
            value={bus.volume_db}
            aria-label="Volume de {bus.name}"
            oninput={(e) => audio.setVolume(bus.name, Number(e.currentTarget.value))}
          />
          <NumberField
            label="Vol"
            value={bus.volume_db}
            step={0.5}
            precision={1}
            suffix="dB"
            min={VOLUME_MIN_DB}
            max={VOLUME_MAX_DB}
            onchange={(v) => audio.setVolume(bus.name, v)}
          />
        </div>

        <div class="fx">
          <span class="sub">Efeitos</span>

          <div class="effect">
            <label class="check">
              <input
                type="checkbox"
                checked={bus.effects.reverb.enabled}
                onchange={(e) => {
                  const on = e.currentTarget.checked;
                  audio.editEffects(bus.name, (fx) => (fx.reverb.enabled = on));
                }}
              />
              <span>Reverberação</span>
            </label>
            {#if bus.effects.reverb.enabled}
              <div class="params">
                <NumberField
                  label="Mix"
                  value={bus.effects.reverb.mix}
                  step={0.05}
                  precision={2}
                  min={0}
                  max={1}
                  onchange={(v) => audio.editEffects(bus.name, (fx) => (fx.reverb.mix = v))}
                />
                <NumberField
                  label="Cauda"
                  value={bus.effects.reverb.decay}
                  step={0.1}
                  precision={1}
                  suffix="s"
                  min={0.1}
                  max={10}
                  onchange={(v) => audio.editEffects(bus.name, (fx) => (fx.reverb.decay = v))}
                />
              </div>
            {/if}
          </div>

          <div class="effect">
            <label class="check">
              <input
                type="checkbox"
                checked={bus.effects.delay.enabled}
                onchange={(e) => {
                  const on = e.currentTarget.checked;
                  audio.editEffects(bus.name, (fx) => (fx.delay.enabled = on));
                }}
              />
              <span>Eco</span>
            </label>
            {#if bus.effects.delay.enabled}
              <div class="params">
                <NumberField
                  label="Tempo"
                  value={bus.effects.delay.time}
                  step={0.01}
                  precision={2}
                  suffix="s"
                  min={0.01}
                  max={2}
                  onchange={(v) => audio.editEffects(bus.name, (fx) => (fx.delay.time = v))}
                />
                <NumberField
                  label="Realim."
                  value={bus.effects.delay.feedback}
                  step={0.05}
                  precision={2}
                  min={0}
                  max={0.95}
                  onchange={(v) => audio.editEffects(bus.name, (fx) => (fx.delay.feedback = v))}
                />
                <NumberField
                  label="Mix"
                  value={bus.effects.delay.mix}
                  step={0.05}
                  precision={2}
                  min={0}
                  max={1}
                  onchange={(v) => audio.editEffects(bus.name, (fx) => (fx.delay.mix = v))}
                />
              </div>
            {/if}
          </div>

          <div class="effect">
            <label class="check">
              <input
                type="checkbox"
                checked={bus.effects.low_pass.enabled}
                onchange={(e) => {
                  const on = e.currentTarget.checked;
                  audio.editEffects(bus.name, (fx) => (fx.low_pass.enabled = on));
                }}
              />
              <span>Passa-baixas</span>
            </label>
            {#if bus.effects.low_pass.enabled}
              <div class="params">
                <NumberField
                  label="Corte"
                  value={bus.effects.low_pass.cutoff_hz}
                  step={10}
                  precision={0}
                  suffix="Hz"
                  min={20}
                  max={20000}
                  onchange={(v) => audio.editEffects(bus.name, (fx) => (fx.low_pass.cutoff_hz = v))}
                />
              </div>
            {/if}
          </div>

          <div class="effect">
            <label class="check">
              <input
                type="checkbox"
                checked={bus.effects.compressor.enabled}
                onchange={(e) => {
                  const on = e.currentTarget.checked;
                  audio.editEffects(bus.name, (fx) => (fx.compressor.enabled = on));
                }}
              />
              <span>Compressor</span>
            </label>
            {#if bus.effects.compressor.enabled}
              <div class="params">
                <NumberField
                  label="Limiar"
                  value={bus.effects.compressor.threshold_db}
                  step={1}
                  precision={0}
                  suffix="dB"
                  min={-100}
                  max={0}
                  onchange={(v) => audio.editEffects(bus.name, (fx) => (fx.compressor.threshold_db = v))}
                />
                <NumberField
                  label="Razão"
                  value={bus.effects.compressor.ratio}
                  step={0.5}
                  precision={1}
                  min={1}
                  max={20}
                  onchange={(v) => audio.editEffects(bus.name, (fx) => (fx.compressor.ratio = v))}
                />
              </div>
            {/if}
          </div>
        </div>
      </section>
    {/each}
  {/if}
</div>

<style>
  .mixer {
    flex: 1;
    display: flex;
    gap: 1px;
    min-width: 0;
    min-height: 0;
    overflow-x: auto;
    overflow-y: hidden;
    background: var(--line);
  }
  .empty {
    margin: auto;
    padding: 12px;
    font-size: var(--fs-12);
    color: var(--text-faint);
    text-align: center;
  }
  .strip {
    flex: none;
    width: 184px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px 8px;
    overflow-y: auto;
    background: var(--bg-1);
  }
  .strip.solo {
    box-shadow: inset 0 2px 0 var(--accent);
  }
  header {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 22px;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-12);
    font-weight: 600;
    color: var(--text);
  }
  .muted .name {
    color: var(--text-faint);
  }
  .tgl {
    width: 22px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: var(--r-ctl);
    background: var(--bg-3);
    color: var(--text-dim);
    transition: color var(--t-fast), background var(--t-fast);
  }
  .tgl:hover {
    color: var(--text);
    background: var(--bg-active);
  }
  .tgl.on {
    color: var(--accent-hi);
    background: var(--accent-soft);
  }
  .fader {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  input[type='range'] {
    width: 100%;
    margin: 0;
    accent-color: var(--accent);
  }
  .fx {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .sub {
    font-size: var(--fs-11);
    font-weight: 600;
    color: var(--text-faint);
  }
  .effect {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .params {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 3px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 20px;
    font-size: var(--fs-12);
    color: var(--text);
    cursor: pointer;
  }
  .check input {
    margin: 0;
    accent-color: var(--accent);
  }
</style>
