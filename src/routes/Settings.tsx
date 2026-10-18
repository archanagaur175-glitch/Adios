import { AppearanceSettings } from "@/components/settings/AppearanceSettings";
import { CacheSettings } from "@/components/settings/CacheSettings";
import { ContactSettings } from "@/components/settings/ContactSettings";
import { ResolverSettings } from "@/components/settings/ResolverSettings";
import { YtdlpSettings } from "@/components/settings/YtdlpSettings";
import { isAndroid } from "@/lib/api";

export default function SettingsPage() {
  return (
    <section className="page flex h-full flex-col gap-6 overflow-y-auto pr-2">
      <h1 className="text-28 font-display">Settings</h1>
      <AppearanceSettings />
      <CacheSettings />
      <ResolverSettings />
      {!isAndroid() && <YtdlpSettings />}
      <ContactSettings />
    </section>
  );
}
