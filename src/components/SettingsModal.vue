<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from "vue";
import { useSettingsStore } from "../stores/settings";
import { useI18n } from "../lib/i18n";
import type { Locale } from "../lib/locale";

const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
}>();

const settings = useSettingsStore();
const { t } = useI18n();

const localeOptions: Array<{ value: Locale; labelKey: string }> = [
  { value: "en", labelKey: "settings.language.en" },
  { value: "zh-CN", labelKey: "settings.language.zh-CN" },
];

const selectedLocale = computed({
  get: () => settings.locale,
  set: (value) => {
    void settings.setLocale(value as Locale);
  },
});

function close() {
  emit("update:modelValue", false);
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && props.modelValue) {
    close();
  }
}

onMounted(() => {
  window.addEventListener("keydown", handleKeydown);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleKeydown);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="props.modelValue" class="modal-backdrop" @click="close">
      <div class="modal" @click.stop>
        <div class="modal-header">
          <h2 class="modal-title">{{ t("settings.title") }}</h2>
        </div>
        <div class="modal-body">
          <div class="field">
            <label>{{ t("settings.language") }}</label>
            <select v-model="selectedLocale">
              <option v-for="option in localeOptions" :key="option.value" :value="option.value">
                {{ t(option.labelKey) }}
              </option>
            </select>
          </div>
        </div>
        <div class="modal-footer">
          <button class="secondary" type="button" @click="close">
            {{ t("settings.modal.close") }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
