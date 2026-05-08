<script>
  $(document).ready(function() {
  	  $('#refresh').click(function() {
         var t = new Date().getTime();
         $('#divcaptcha').html('<img src="{site_url}/image.php?to=captcha&t=' + t + '" alt="Captcha" />');
      });
  });
</script>
<form action="{site_url}/index.php?dn=mail" method="post">
<div class="forms">
    <fieldset class="standart">
    <legend>{email_name}</legend>
        <strong>|</strong>
        <input class="width" name="sendnames" type="text" value="{uname}" />
    </fieldset>
    <fieldset class="standart">
    <legend>{email}:</legend>
        <strong>|</strong>
        <input class="width" name="sendmails" type="text" value="{umail}" />
    </fieldset>
    <fieldset class="standart">
    <legend>{email_text}:</legend>
        <textarea cols="40" class="width" rows="10" name="sendtexts"></textarea>
    </fieldset>
    <!--if:captcha:yes-->
    <fieldset class="standart">
    <legend>{captcha}</legend>
    <table class="wpc_100">
    <tbody>
        <tr>
            <td class="wpc_100">
                <strong>|</strong>
                <input class="width" id="captcha" name="captcha" type="text" maxlength="5" />
            </td>
            <td class="ac va pad">
                <div id="divcaptcha"><img src="{site_url}/image.php?to=captcha" alt="" /></div>
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
        <strong>|</strong>
        <input class="width" id="respon" name="respon" size="30" type="text" />
        <input name="cid" type="hidden" value="{cid}" />
    </fieldset>
    <!--if-->
    <div class="pad ac">
        <input name="to" type="hidden" value="send" />
        <button type="submit" class="sub">{email_send}</button>
    </div>
    <div class="clear"></div>
</div>
</form>
