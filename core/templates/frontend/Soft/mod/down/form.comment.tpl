<script src="{site_url}/javascript/jquery.form.js"></script>
<script>
  $(document).ready(function() {
      $.last = {last};
      {smiliearray}
      {template}
  	  $("#comtext").comment({activejs});
  	  $('#comtext').textarearesizer();
  	  $('#refresh').click(function() {
         var t = new Date().getTime();
         $('#divcaptcha').html('<img src="{site_url}/image.php?to=captcha&t=' + t + '" alt="" />');
      });
      $('#commment-form').submit(function() {
         $('#commment-form input, textarea').removeClass('error-input').addClass('width');
         $error = false;
         $.check = new Array();
         $.check['comname'] = new Array('comname',{comname});
         $.check['comtext'] = new Array('comtext',{comsize});
         <!--if:captcha:yes-->$.check['captcha'] = new Array('captcha',5);<!--if-->
         <!--if:control:yes-->$.check['respon'] = new Array('respon',0);<!--if-->
         for(i in $.check) {
             var id = $.check[i][0], val = $.check[i][1];
             if (val == 0) {
                if ($("#" + id) != "undefined" && $("#" + id).val().length == 0) {
                 	$error = true;
                 	$("#" + id).removeClass('width').addClass('error-input');
                 	$("#" + id).focus(function(){
                      $(this).removeClass('error-input').addClass('width');
                    });
                }
             }
             if (val > 0) {
                if ($("#" + id) != "undefined" && $("#" + id).val().length == 0 || $("#" + id) != "undefined" && $("#" + id).val().length > val) {
                 	$error = true;
                    $("#" + id).removeClass('width').addClass('error-input');
                    $("#" + id).focus(function(){
                      $(this).removeClass('error-input').addClass('width');
                    });
                }
             }
         }
         if ($error) {
         	 return false;
         }
         <!--if:ajax:yes-->
         $('#sendbox').show();
         $("#errorbox").html('');
         var value = $(this).serialize();
          $.ajax({
           cache:false,
           type:'POST',
           url:'{site_url}/index.php?dn=down&re=comment&ajax=1&ct=' + $.last,
           data:value,
           error: function(data) { $('#commment-form').submit(); },
           success: function(data) {
             $("#sendbox").hide();
             if (data.match(/^<!--ok ([0-9]+)-->/)) {
                 var pt = data.match(/^<!--ok ([0-9]+)-->/);
                 if (pt) {
                 	$.last = pt[1];
                 }
                 $("#ajaxbox").append(data).show();
             } else {
             	 $("#errorbox").html(data);
             }
          }
         })
        return false;
        <!--if-->
      });
  });
</script>
<div class="commentsend" id="sendbox" style="display:none">
    <img src="{site_url}/temp/{site_temp}/images/progress.gif" alt="{all_sends}" /> <span class="sendtext">{all_sends}...</span>
</div>
<form action="{site_url}/index.php?dn=down&amp;re=comment" method="post" id="commment-form">
<div class="comment">
    <!--if:uname:yes-->
    <fieldset class="standart">
    <legend>{comment_name}</legend>
        <strong>|</strong><input class="width" name="comname" id="comname" size="35" type="text" value="{uname}" />
    </fieldset>   
    <!--if--> 
    <!--if:uname:no-->
    <input name="comname" id="comname" type="hidden" value="{uname}" />
    <!--if-->
    <fieldset class="standart">
    <legend>{all_text}</legend>
        <textarea class="width" cols="60" rows="5" name="comtext" id="comtext"></textarea>
    </fieldset>
    <!--if:captcha:yes-->
    <fieldset class="standart">
    <legend>Captcha</legend>
        <table class="wpc_100">
        <tbody>
            <tr>
                <td class="wpc_90">
                    <strong>|</strong><input class="width" id="captcha" name="captcha" type="text" maxlength="5" />
                </td>
                <td class="ac va pad">
                    <div id="divcaptcha"><img src="{site_url}/image.php?to=captcha" alt="Captcha" /></div>
                </td>
                <td class="ac va pad">
                    <button type="button" id="refresh" class="sub">{all_refresh}</button>
                </td>
            </tr>
        </tbody>
        </table>
    </fieldset>
    <!--if-->
    <!--if:control:yes-->
    <fieldset class="standart">
    <legend>{control_word}</legend>
        <p>{control}</p>
        <strong>|</strong><input class="width" id="respon" name="respon" size="30" type="text" />
        <input name="cid" type="hidden" value="{cid}" />
    </fieldset>
    <!--if-->
    <div class="pad ac">
        <input name="id" value="{id}" type="hidden" />
        <button type="submit" class="sub">{comment_add_button}</button>
    </div>
</div>
</form>
