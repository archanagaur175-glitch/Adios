import { AppearanceSettings } from "@/components/settings/AppearanceSettings";
import { ContactSettings } from "@/components/settings/ContactSettings";
import { ResolverSettings } from "@/components/settings/ResolverSettings";
import { YtdlpSettings } from "@/components/settings/YtdlpSettings";

export default function SettingsPage() {
  return (
    <section className="page flex h-full flex-col gap-6 overflow-y-auto pr-2">
      <h1 className="text-28 font-display">Settings</h1>
      <AppearanceSettings />
      <ResolverSettings />
      <YtdlpSettings />
      <ContactSettings />
    </section>
  );
}
