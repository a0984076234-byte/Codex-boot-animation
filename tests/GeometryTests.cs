using System;
using System.IO;
using System.Web.Script.Serialization;
using DragonCodexBoot;

internal static class GeometryTests {
 private static int checks;
 private static void Check(bool ok,string message) { if(!ok)throw new Exception(message);checks++; }
 private static Config Clone(Config c) { var s=new JavaScriptSerializer();return s.Deserialize<Config>(s.Serialize(c)); }
 private static void Reject(Config original,Action<Config> change,string label) {
  Config copy=Clone(original);change(copy);bool rejected=false;
  try { Entry.ValidateConfig(copy); } catch(Exception) { rejected=true; }
  Check(rejected,"Invalid config accepted: "+label);
 }
 public static int Main(string[] args) {
  try {
   Config c=new JavaScriptSerializer().Deserialize<Config>(File.ReadAllText(args[0]));Entry.ValidateConfig(c);
   Check(c.AutoReplaceEntrypoints&&c.ScanAllLocalDrives,"Public defaults enable first-run integration");
   Check(new Config().AutoReplaceEntrypoints&&new Config().ScanAllLocalDrives,"Missing legacy settings have enabled defaults");
   string legacy=File.ReadAllText(args[0]).Replace("\"AutoReplaceEntrypoints\": true,","").Replace("\"ScanAllLocalDrives\": true,","");
   Check(new JavaScriptSerializer().Deserialize<Config>(legacy).AutoReplaceEntrypoints,"Legacy JSON retains default");
   Check(!new JavaScriptSerializer().Deserialize<Config>("{\"AutoReplaceEntrypoints\":false}").AutoReplaceEntrypoints,"Explicit opt-out is retained");
   Check(Geometry.At(c.ScreenFrames,-1)==c.ScreenFrames[0],"First frame clamp");
   Check(Geometry.At(c.ScreenFrames,999)==c.ScreenFrames[c.ScreenFrames.Count-1],"Last frame clamp");
   double mid=(c.ScreenFrames[0].Time+c.ScreenFrames[1].Time)/2;
   var frame=Geometry.At(c.ScreenFrames,mid);
   Check(Math.Abs(frame.X-(c.ScreenFrames[0].X+c.ScreenFrames[1].X)/2)<1e-10,"Interpolated x");
   Bounds monitor=new Bounds(-2560,-1440,2560,1440);
   Bounds centered=Geometry.Centered(monitor,1920,1080);
   Check(centered.Left==-2240&&centered.Top==-1260&&centered.Width==1920&&centered.Height==1080,"Negative monitor center");
   Bounds full=Geometry.Dest(c.ScreenFrames,c.TransitionEnd,1200,900,1);
   Check(full.Left==0&&full.Top==0&&full.Right==1200&&full.Bottom==900,"Final non-16:9 viewport fills");
   Reject(c,x=>x.TransitionEnd=x.TransitionStart,"zero fade length");
   Reject(c,x=>x.HoldAt=x.TransitionStart,"hold after fade");
   Reject(c,x=>x.Volume=Double.NaN,"NaN volume");
   Reject(c,x=>x.Volume=1.01,"volume range");
   Reject(c,x=>x.TransitionStart=Double.PositiveInfinity,"infinite time");
   Reject(c,x=>x.ScreenFrames[0].X=Double.NaN,"NaN position");
   Reject(c,x=>x.ScreenFrames[0].Width=0,"zero screen width");
   Reject(c,x=>x.ScreenFrames[0].Time=-1,"negative keyframe time");
   Reject(c,x=>x.ScreenFrames[1].Time=x.ScreenFrames[0].Time,"duplicate keyframe time");
   Reject(c,x=>x.ScreenFrames[0]=null,"null keyframe");
   Reject(c,x=>x.ProcessNames[0]=" ","blank process");
   Reject(c,x=>x.AppLaunch="shell:AppsFolder\\app!App extra","shell argument injection");
   Reject(c,x=>x.PlayerWidth=10,"unusable player width");
   Console.WriteLine("PASS: "+checks+" geometry/configuration assertions. No desktop UI opened.");
   return 0;
  } catch(Exception e) { Console.Error.WriteLine(e);return 1; }
 }
}
