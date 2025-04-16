package local.walkers;

import android.os.Bundle;
import android.util.Log;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewGroup;

import androidx.core.graphics.Insets;
import androidx.core.view.DisplayCutoutCompat;
import androidx.core.view.ViewCompat;
import androidx.core.view.WindowCompat;
import androidx.core.view.WindowInsetsCompat;

import com.google.androidgamesdk.GameActivity;

public class MainActivity extends GameActivity {
  static {
    System.loadLibrary("main");
  }

  @Override
  protected void onCreate(Bundle savedInstanceState) {
      // Shrink view so it does not get covered by insets.

      View content = getWindow().getDecorView().findViewById(android.R.id.content);
      ViewCompat.setOnApplyWindowInsetsListener(content, (v, windowInsets) -> {
        Insets insets = windowInsets.getInsets(WindowInsetsCompat.Type.systemBars());

        ViewGroup.MarginLayoutParams mlp = (ViewGroup.MarginLayoutParams) v.getLayoutParams();
        mlp.topMargin = insets.top;
        mlp.leftMargin = insets.left;
        mlp.bottomMargin = insets.bottom;
        mlp.rightMargin = insets.right;
        v.setLayoutParams(mlp);

        return WindowInsetsCompat.CONSUMED;
      });

      WindowCompat.setDecorFitsSystemWindows(getWindow(), true);

      super.onCreate(savedInstanceState);
  }

  @Override
  public boolean onTouchEvent(MotionEvent event) {
      // Offset the location so it fits the view with margins caused by insets.

      int[] location = new int[2];
      findViewById(android.R.id.content).getLocationOnScreen(location);
      event.offsetLocation(-location[0], -location[1]);
      return super.onTouchEvent(event);
  }
}
