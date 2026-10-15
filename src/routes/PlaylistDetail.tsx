import { useParams } from "react-router";
import { PagePlaceholder } from "@/components/common/PagePlaceholder";

export default function PlaylistDetailPage() {
  const { id } = useParams();
  return <PagePlaceholder title={`Playlist ${id ?? ""}`} />;
}
